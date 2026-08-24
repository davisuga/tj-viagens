/**
 * Records the TJ-Viagens golden path with two browser contexts:
 *  - staff (desktop 1280x800), supplier (iPhone-ish 390x844 mobile emulation)
 * Outputs: ../demo/out/video/*.webm (b-roll for the pitch's Ato III) and
 *          ../demo/out/shots/NN-*.png (visual self-audit).
 * Prereqs: docker compose db up, seeded DB (cargo run --bin seed), API on :3001,
 *          web dev server on :5173. Window length doesn't matter — the script
 *          force-closes the bidding window via SQL for a tight recording.
 * Run: cd web && bun run demo:record
 */
import { execSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { chromium, type BrowserContext, type Page } from 'playwright';

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

async function main(): Promise<void> {
  mkdirSync(`${OUT}/shots`, { recursive: true });
  writeFileSync(`${OUT}/eticket-demo.pdf`, '%PDF-1.4 demo e-ticket TJ-Viagens\n');

  const browser = await chromium.launch();
  const staffCtx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    recordVideo: { dir: `${OUT}/video/staff`, size: { width: 1280, height: 800 } },
  });
  const supplierCtx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    recordVideo: { dir: `${OUT}/video/supplier`, size: { width: 390, height: 844 } },
  });
  const winnerCtx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    recordVideo: { dir: `${OUT}/video/winner`, size: { width: 390, height: 844 } },
  });

  // Vite's dev-server dependency pre-bundling can stall the FIRST navigation to
  // each not-yet-visited route (esp. right after `bun add` changed the lockfile),
  // well past Playwright's 30s default. Bump it repo-wide as cheap insurance —
  // every subsequent visit is fast once the module graph is warm.
  for (const ctx of [staffCtx, supplierCtx, winnerCtx]) ctx.setDefaultTimeout(60_000);

  // ── Staff: dashboard with seeded KPIs ─────────────────────────────
  const staff = await staffCtx.newPage();
  await login(staff, 'servidor@tjrr.jus.br', /\/$/);
  await staff.getByText('Economia acumulada').waitFor();
  await snap(staff, 'staff-dashboard-kpis');

  // ── Staff: new demand ─────────────────────────────────────────────
  await staff.getByRole('link', { name: 'Nova cotação' }).click();
  await staff.fill('#passengerName', 'Maria da Silva');
  await staff.fill('#passengerCpf', '123.456.789-09');
  await staff.fill('#passengerBirth', '1985-04-12');
  await staff.fill('#origin', 'BVB');
  await staff.fill('#destination', 'BSB');
  await staff.fill('#departureAt', '2026-09-15T08:00');
  await staff.fill('#referenceFlight', 'LA-4001');
  await staff.fill('#referencePrice', '1.850,00');
  await snap(staff, 'staff-new-demand-form');
  await staff.getByRole('button', { name: 'Criar rascunho' }).click();
  // Staff is already on /cotacoes/nova (the form's own route) at this point, which
  // itself matches a bare /\/cotacoes\// pattern — waitForURL would then resolve
  // immediately against the CURRENT url instead of the post-create navigation to
  // /cotacoes/{uuid}, leaving quotationId as the literal string "nova". Anchor on
  // the UUID shape so it only matches the real detail page.
  await staff.waitForURL(/\/cotacoes\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  const quotationId = staff.url().split('/').pop() ?? '';
  await staff.getByRole('button', { name: 'Abrir cotação' }).waitFor();
  await snap(staff, 'staff-draft-detail');

  // ── Staff: open the dispute (confirm dialog) ──────────────────────
  await staff.getByRole('button', { name: 'Abrir cotação' }).click();
  await staff.getByRole('button', { name: 'Confirmar abertura' }).click();
  await staff.getByText('propostas recebidas').waitFor();
  await snap(staff, 'staff-open-countdown');

  // ── Supplier 1 (mobile): notification -> blind bid ────────────────
  const supplier = await supplierCtx.newPage();
  await login(supplier, 'contato@voaroraima.com.br', /\/fornecedor$/);
  // waitForURL above resolves on the client-side route push, before the
  // /suppliers/me query resolves — without this wait the shot always caught the
  // "Carregando…" placeholder instead of the credentialing checklist. Dev-mode
  // React 19 StrictMode also double-invokes the mount effect, which can flip this
  // freshly-mounted page loading -> data -> loading -> data; re-confirm after a
  // short settle so the shot doesn't land on that transient reversion (measured
  // ~300ms in practice — 500ms leaves margin).
  await supplier.getByText('Credenciamento').waitFor();
  await supplier.waitForTimeout(500);
  await supplier.getByText('Credenciamento').waitFor();
  await snap(supplier, 'supplier-home-mobile');
  await supplier.goto(`${WEB}/fornecedor/cotacoes/${quotationId}`);
  await supplier.getByText('Tempo restante').waitFor();
  await snap(supplier, 'supplier-bid-countdown-mobile');
  await supplier.fill('#price', '1.523,00');
  await supplier.fill('#flight', 'G3-1720 · 15/09 08:15');
  await supplier.getByRole('button', { name: 'Enviar proposta' }).click();
  await supplier.getByText('Proposta registrada').first().waitFor();
  await snap(supplier, 'supplier-bid-submitted');

  // ── Supplier 2 (winner-to-be) bids lower ──────────────────────────
  const winner = await winnerCtx.newPage();
  await login(winner, 'contato@amazoniaviagens.com.br', /\/fornecedor$/);
  await winner.goto(`${WEB}/fornecedor/cotacoes/${quotationId}`);
  await winner.fill('#price', '1.499,00');
  await winner.fill('#flight', 'G3-1720 · 15/09 08:15');
  await winner.getByRole('button', { name: 'Enviar proposta' }).click();
  await winner.getByText('Proposta registrada').first().waitFor();

  // ── Staff: sealed live count ──────────────────────────────────────
  await staff.getByText('2', { exact: true }).first().waitFor();
  await snap(staff, 'staff-live-count-sealed');

  // ── Force-close the window (server clock) -> ranking ──────────────
  sql(`UPDATE quotations SET closes_at = now() - interval '1 second' WHERE id = '${quotationId}'`);
  await staff.reload();
  await staff.getByText('menor para maior').waitFor();
  await snap(staff, 'staff-ranking-preselected');

  // ── One-click award ───────────────────────────────────────────────
  await staff.getByRole('button', { name: 'Declarar vencedora e emitir OS' }).click();
  await staff.getByText('Aguardando e-ticket').waitFor();
  await snap(staff, 'staff-awarded-waiting-ticket');

  // ── Winner: banner + e-ticket upload ──────────────────────────────
  await winner.reload();
  await winner.getByText('Sua proposta venceu').waitFor();
  await snap(winner, 'winner-banner-mobile');
  await winner.fill('#ticketPrice', '1.499,00');
  await winner.setInputFiles('#ticketFile', `${OUT}/eticket-demo.pdf`);
  await winner.getByRole('button', { name: 'Anexar e-ticket' }).click();
  await winner.getByText('E-ticket enviado').first().waitFor();
  await snap(winner, 'winner-ticket-uploaded');

  // ── Staff: conference -> complete -> economy ──────────────────────
  await staff.reload();
  await staff.getByText('Conferência do e-ticket').waitFor();
  await snap(staff, 'staff-ticket-conference');
  await staff.getByRole('button', { name: 'Confirmar e concluir' }).click();
  // 'abaixo da referência' already renders in the prior (TICKETED) screenshot —
  // the economy card doesn't depend on ticket confirmation — so waiting on it here
  // resolves immediately, racing ahead of the COMPLETED transition. Wait for the
  // status badge to actually flip instead. exact:true is required — a plain
  // substring match also hits the "Cotação concluída." success toast.
  await staff.getByText('Concluída', { exact: true }).waitFor();
  await snap(staff, 'staff-economy-and-audit');

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
  await supplierCtx.close();
  await winnerCtx.close();
  await browser.close();
  console.log(`🎬 vídeos em ${OUT}/video/{staff,supplier,winner} · ${shot} screenshots em ${OUT}/shots`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
