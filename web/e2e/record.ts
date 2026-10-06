/**
 * Records the TJ-Viagens golden path with three browser contexts:
 *  - staff (desktop 1280x800) and two agencies (iPhone-ish 390x844 mobile emulation)
 * Outputs: ../demo/out/video/*.webm (b-roll) and ../demo/out/shots/NN-*.png
 *          (visual self-audit).
 * Prereqs: docker compose db up, seeded DB (cargo run --bin seed), API on :3001,
 *          web dev server on :5173. Window length doesn't matter — the script
 *          force-closes the bidding window via SQL for a tight recording.
 * Run: cd web && bun run demo:record
 *
 * The story is the SGA's everyday trade-off out of Roraima: Amazônia bids the
 * lowest price with a connection and 8h50 of travel; Voa Roraima bids R$ 24 more
 * for 5h40. The servidor awards the shorter trip, with a reason the API verifies
 * against the bids, and the dossiê prints the comparison.
 */
import { execSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { chromium, type Page } from 'playwright';

const WEB = process.env.WEB_URL ?? 'http://localhost:5173';
const API = process.env.API_URL ?? 'http://localhost:3001';
const OUT = '../demo/out';

let shot = 0;
async function snap(page: Page, name: string, fullPage = false): Promise<void> {
  shot += 1;
  const path = `${OUT}/shots/${String(shot).padStart(2, '0')}-${name}.png`;
  await page.screenshot({ path, fullPage });
  console.log(`📸 ${path}`);
}

async function login(page: Page, email: string, landing: RegExp): Promise<void> {
  await page.goto(`${WEB}/login`);
  await page.fill('#email', email);
  await page.fill('#password', 'demo1234');
  await page.getByRole('button', { name: 'Entrar' }).click();
  await page.waitForURL(landing);
}

function sql(statement: string): void {
  execSync(
    `docker compose exec -T db psql -U tj -d tjviagens -c "${statement.replace(/"/g, '\\"')}"`,
    { cwd: '..', stdio: 'inherit' },
  );
}

async function bid(
  page: Page,
  bidData: { price: string; airline: string; flights: string; duration: string },
): Promise<void> {
  await page.fill('#price', bidData.price);
  await page.fill('#airline', bidData.airline);
  await page.fill('#flight', bidData.flights);
  // Departure comes pre-filled with the requested one; connections default to 1.
  await page.fill('#duration', bidData.duration);
}

async function main(): Promise<void> {
  mkdirSync(`${OUT}/shots`, { recursive: true });
  writeFileSync(`${OUT}/eticket-demo.pdf`, '%PDF-1.4 demo e-ticket TJ-Viagens\n');
  // A month out, so the demand never reads as a trip in the past.
  const departureDay = new Date(Date.now() + 30 * 86_400_000).toISOString().slice(0, 10);

  const browser = await chromium.launch();
  const staffCtx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    recordVideo: { dir: `${OUT}/video/staff`, size: { width: 1280, height: 800 } },
  });
  const mobile = (dir: string) =>
    browser.newContext({
      viewport: { width: 390, height: 844 },
      isMobile: true,
      hasTouch: true,
      recordVideo: { dir: `${OUT}/video/${dir}`, size: { width: 390, height: 844 } },
    });
  const fastCtx = await mobile('supplier-fast');
  const cheapCtx = await mobile('supplier-cheap');

  // Vite's dev-server dependency pre-bundling can stall the FIRST navigation to
  // each not-yet-visited route, well past Playwright's 30s default.
  for (const ctx of [staffCtx, fastCtx, cheapCtx]) ctx.setDefaultTimeout(60_000);

  // ── Staff: dashboard with seeded KPIs ─────────────────────────────
  const staff = await staffCtx.newPage();
  await login(staff, 'servidor@tjrr.jus.br', /\/$/);
  await staff.getByText('Economia acumulada').waitFor();
  await snap(staff, 'staff-dashboard-kpis');

  // ── Staff: certidões at a glance (credenciado ≠ habilitado) ───────
  await staff.getByRole('link', { name: 'Fornecedores' }).click();
  await staff.getByText('Credenciamento de fornecedores').waitFor();
  await staff.getByText('CRF do FGTS vencida').waitFor();
  await snap(staff, 'staff-suppliers-certidoes');
  await staff.goto(`${WEB}/`);

  // ── Staff: new demand, from its SEI process ───────────────────────
  await staff.getByRole('link', { name: 'Nova cotação' }).click();
  await staff.fill('#seiProcess', '0004513-12.2026.8.23.8000');
  await staff.fill('#passengerName', 'Maria da Silva');
  await staff.fill('#passengerCpf', '123.456.789-09');
  await staff.fill('#passengerBirth', '1985-04-12');
  await staff.fill('#origin', 'BVB');
  await staff.fill('#destination', 'BSB');
  await staff.fill('#departureAt', `${departureDay}T08:00`);
  await staff.fill('#referenceFlight', 'LA-4001');
  await staff.fill('#referencePrice', '1.850,00');
  await snap(staff, 'staff-new-demand-form');
  await staff.getByRole('button', { name: 'Criar rascunho' }).click();
  // Anchor on the UUID: the form's own /cotacoes/nova would match a looser pattern.
  await staff.waitForURL(/\/cotacoes\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  const quotationId = staff.url().split('/').pop() ?? '';
  await staff.getByRole('button', { name: 'Abrir cotação' }).waitFor();
  await snap(staff, 'staff-draft-detail');

  // ── Staff: open the dispute (confirm dialog) ──────────────────────
  await staff.getByRole('button', { name: 'Abrir cotação' }).click();
  await staff.getByRole('button', { name: 'Confirmar abertura' }).click();
  await staff.getByText('propostas recebidas').waitFor();
  await snap(staff, 'staff-open-countdown');

  // ── Voa Roraima (mobile): CRF about to expire, shorter itinerary ──
  const fast = await fastCtx.newPage();
  await login(fast, 'contato@voaroraima.com.br', /\/fornecedor$/);
  // Dev-mode StrictMode can flip a fresh page loading -> data -> loading -> data;
  // re-confirm after a short settle so the shot doesn't land on the reversion.
  await fast.getByText('vence em 6 dias').waitFor();
  await fast.waitForTimeout(500);
  await fast.getByText('vence em 6 dias').waitFor();
  await snap(fast, 'supplier-home-certidao-expiring');
  await fast.goto(`${WEB}/fornecedor/cotacoes/${quotationId}`);
  await fast.getByText('Tempo restante').waitFor();
  await bid(fast, { price: '1.523,00', airline: 'LATAM', flights: 'LA-3341 / LA-3020', duration: '5:40' });
  await snap(fast, 'supplier-bid-form-filled', true);
  await fast.getByRole('button', { name: 'Enviar proposta' }).click();
  await fast.getByText('Proposta registrada').first().waitFor();
  await snap(fast, 'supplier-bid-submitted');

  // ── Amazônia: lowest price, 8h50 via Manaus ───────────────────────
  const cheap = await cheapCtx.newPage();
  await login(cheap, 'contato@amazoniaviagens.com.br', /\/fornecedor$/);
  await cheap.goto(`${WEB}/fornecedor/cotacoes/${quotationId}`);
  await cheap.getByText('Tempo restante').waitFor();
  await bid(cheap, { price: '1.499,00', airline: 'GOL', flights: 'G3-1720 / G3-1450', duration: '8:50' });
  await cheap.getByRole('button', { name: 'Enviar proposta' }).click();
  await cheap.getByText('Proposta registrada').first().waitFor();

  // ── Staff: sealed live count ──────────────────────────────────────
  await staff.getByText('2', { exact: true }).first().waitFor();
  await snap(staff, 'staff-live-count-sealed');

  // ── Force-close the window (server clock) -> ranking ──────────────
  sql(`UPDATE quotations SET closes_at = now() - interval '1 second' WHERE id = '${quotationId}'`);
  await staff.reload();
  await staff.getByText('menor para maior').waitFor();
  await staff.getByText('mais rápida').waitFor();
  await snap(staff, 'staff-ranking-preselected', true);

  // ── Staff: award the shorter trip, with a verified reason ─────────
  await staff.getByRole('radio', { name: 'Selecionar Voa Roraima Turismo LTDA' }).check();
  await staff.getByText('Proposta mais cara que a de menor preço').waitFor();
  await staff.fill('#justification', 'Chega 3h10 antes: a servidora tem audiência às 14h em Brasília.');
  await snap(staff, 'staff-award-deviation', true);
  await staff.getByRole('button', { name: 'Declarar vencedora e emitir OS' }).click();
  await staff.getByRole('button', { name: 'Confirmar adjudicação' }).waitFor();
  // Let the dialog's entrance spring settle so the shot isn't mid-animation.
  await staff.waitForTimeout(450);
  await snap(staff, 'staff-award-confirm');
  await staff.getByRole('button', { name: 'Confirmar adjudicação' }).click();
  await staff.getByText('Aguardando e-ticket').waitFor();
  await snap(staff, 'staff-awarded-waiting-ticket', true);

  // ── Winner (Voa Roraima): banner + e-ticket upload ────────────────
  await fast.reload();
  await fast.getByText('Sua proposta venceu').waitFor();
  await snap(fast, 'winner-banner-mobile');
  await fast.fill('#ticketPrice', '1.523,00');
  await fast.setInputFiles('#ticketFile', `${OUT}/eticket-demo.pdf`);
  await fast.getByRole('button', { name: 'Anexar e-ticket' }).click();
  await fast.getByText('E-ticket enviado').first().waitFor();
  await snap(fast, 'winner-ticket-uploaded');

  // ── Staff: conference -> complete -> economy ──────────────────────
  await staff.reload();
  await staff.getByText('Conferência do e-ticket').waitFor();
  await snap(staff, 'staff-ticket-conference');
  await staff.getByRole('button', { name: 'Confirmar e concluir' }).click();
  await staff.getByRole('button', { name: 'Concluir cotação' }).click();
  // exact:true — a substring match also hits the "Cotação concluída." toast.
  await staff.getByText('Concluída', { exact: true }).waitFor();
  await staff.getByRole('button', { name: 'Concluir cotação' }).waitFor({ state: 'detached' });
  await staff.waitForTimeout(450);
  await snap(staff, 'staff-economy-and-audit', true);

  // ── Printable pages (token via localStorage) ──────────────────────
  const token = await staff.evaluate(() => localStorage.getItem('tj_token'));
  await staff.goto(`${API}/quotations/${quotationId}/service-order?token=${token}`);
  await snap(staff, 'printable-service-order', true);
  await staff.goto(`${API}/quotations/${quotationId}/report?token=${token}`);
  await snap(staff, 'printable-report', true);

  // ── Final dashboard ───────────────────────────────────────────────
  await staff.goto(`${WEB}/`);
  await staff.getByText('Economia acumulada').waitFor();
  await snap(staff, 'staff-dashboard-final');

  await staffCtx.close();
  await fastCtx.close();
  await cheapCtx.close();
  await browser.close();
  console.log(`🎬 vídeos em ${OUT}/video/{staff,supplier-fast,supplier-cheap} · ${shot} screenshots em ${OUT}/shots`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
