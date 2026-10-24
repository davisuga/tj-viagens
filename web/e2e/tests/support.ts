import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, type Browser, type BrowserContext, type Page } from '@playwright/test';

export const API = 'http://localhost:3101';
export const WEB = 'http://localhost:5174';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const E2E_DATABASE_URL = 'postgresql://tj:tj@localhost:5433/tjviagens_e2e';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const DETAIL_URL = /\/cotacoes\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

// Seeded accounts (api/src/bin/seed.rs), password demo1234.
export const STAFF = 'servidor@tjrr.jus.br';
/** Credenciada, every certidão valid, CRF do FGTS expiring in 6 days. */
export const VOA = 'contato@voaroraima.com.br';
export const AMAZONIA = 'contato@amazoniaviagens.com.br';
/** Credenciada, but the CRF do FGTS lapsed 3 days ago: can bid, cannot win. */
export const RIO_BRANCO = 'contato@riobrancotur.com.br';

/** Fresh demo state. The seed truncates and repopulates the e2e database. */
export function reseed(): void {
  execFileSync(path.join(ROOT, 'api/target/debug/seed'), {
    cwd: path.join(ROOT, 'api'),
    env: { ...process.env, DATABASE_URL: E2E_DATABASE_URL },
    stdio: 'pipe',
  });
}

/** The server clock decides when bidding ends; tests move closes_at instead of
 *  waiting out the hour. */
export function closeBidding(quotationId: string): void {
  if (!UUID.test(quotationId)) throw new Error(`not a quotation id: ${quotationId}`);
  execFileSync(
    'docker',
    [
      'compose', 'exec', '-T', 'db', 'psql', '-U', 'tj', '-d', 'tjviagens_e2e', '-v', 'ON_ERROR_STOP=1',
      '-c', `UPDATE quotations SET closes_at = now() - interval '1 second' WHERE id = '${quotationId}'`,
    ],
    { cwd: ROOT, stdio: 'pipe' },
  );
}

/** Agencies quote from their phones. */
export function phone(browser: Browser): Promise<BrowserContext> {
  return browser.newContext({
    baseURL: WEB,
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
  });
}

export async function login(page: Page, email: string): Promise<void> {
  await page.goto('/login');
  await page.fill('#email', email);
  await page.fill('#password', 'demo1234');
  await page.getByRole('button', { name: 'Entrar' }).click();
  await page.waitForURL((url) => !url.pathname.startsWith('/login'));
}

/** Same call the app makes, with the session the page holds. */
export async function apiAs(page: Page, pathname: string): Promise<{ status: number; body: string }> {
  return page.evaluate(
    async ([base, p]) => {
      const res = await fetch(`${base}${p}`, {
        headers: { Authorization: `Bearer ${localStorage.getItem('tj_token')}` },
      });
      return { status: res.status, body: await res.text() };
    },
    [API, pathname] as const,
  );
}

/** A month out, so the demand never reads as a trip in the past. */
export const DEPARTURE_DAY = new Date(Date.now() + 30 * 86_400_000).toISOString().slice(0, 10);
export const SEI = '0004513-12.2026.8.23.8000';

export async function fillDemand(page: Page, sei: string): Promise<void> {
  await page.goto('/cotacoes/nova');
  await page.fill('#seiProcess', sei);
  await page.fill('#passengerName', 'Maria da Silva');
  await page.fill('#passengerCpf', '123.456.789-09');
  await page.fill('#passengerBirth', '1985-04-12');
  await page.fill('#origin', 'BVB');
  await page.fill('#destination', 'BSB');
  await page.fill('#departureAt', `${DEPARTURE_DAY}T08:00`);
  await page.fill('#referenceFlight', 'LA-4001');
  await page.fill('#referencePrice', '1.850,00');
}

/** Staff creates the demand and opens the dispute; returns the quotation id. */
export async function openQuotation(page: Page, sei = SEI): Promise<string> {
  await fillDemand(page, sei);
  await page.getByRole('button', { name: 'Criar rascunho' }).click();
  await page.waitForURL(DETAIL_URL);
  const id = new URL(page.url()).pathname.split('/').pop() ?? '';
  await page.getByRole('button', { name: 'Abrir cotação' }).click();
  await page.getByRole('button', { name: 'Confirmar abertura' }).click();
  await expect(page.getByText('propostas recebidas')).toBeVisible();
  return id;
}

export type Bid = {
  price: string;
  airline: string;
  flights: string;
  duration: string;
  /** Option label in the Conexões select; the form defaults to "1 conexão". */
  connections?: 'Direto (sem conexão)' | '1 conexão' | '2 conexões';
};

export async function placeBid(page: Page, quotationId: string, bid: Bid): Promise<void> {
  await page.goto(`/fornecedor/cotacoes/${quotationId}`);
  await expect(page.getByText('Tempo restante para propostas')).toBeVisible();
  await page.fill('#price', bid.price);
  await page.fill('#airline', bid.airline);
  await page.fill('#flight', bid.flights);
  await page.fill('#duration', bid.duration);
  if (bid.connections) {
    await page.getByRole('combobox', { name: 'Conexões' }).click();
    await page.getByRole('option', { name: bid.connections }).click();
  }
  await page.getByRole('button', { name: 'Enviar proposta' }).click();
  await expect(page.getByText('Proposta registrada').first()).toBeVisible();
}

/** Logs an agency in on a phone-sized context and places its bid. */
export async function agencyBids(
  browser: Browser,
  email: string,
  quotationId: string,
  bid: Bid,
): Promise<Page> {
  const page = await (await phone(browser)).newPage();
  await login(page, email);
  await placeBid(page, quotationId, bid);
  return page;
}

// The SGA's everyday trade-off out of Roraima.
export const CHEAP_LONG: Bid = { price: '1.499,00', airline: 'GOL', flights: 'G3-1720 / G3-1450', duration: '8:50' };
export const PRICIER_SHORTER: Bid = { price: '1.523,00', airline: 'LATAM', flights: 'LA-3341 / LA-3020', duration: '5:40' };
export const CHEAPEST_DIRECT: Bid = {
  price: '1.450,00',
  airline: 'GOL',
  flights: 'G3-1601',
  duration: '3:15',
  connections: 'Direto (sem conexão)',
};
