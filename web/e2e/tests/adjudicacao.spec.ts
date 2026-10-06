import { expect, test } from '@playwright/test';
import {
  AMAZONIA, API, CHEAPEST_DIRECT, CHEAP_LONG, PRICIER_SHORTER, RIO_BRANCO, SEI, STAFF, VOA,
  agencyBids, apiAs, closeBidding, login, openQuotation, reseed,
} from './support';

test.beforeEach(() => reseed());

test('awarding the shorter trip over the cheapest, past a supplier with a lapsed CRF', async ({
  page,
  browser,
}) => {
  await login(page, STAFF);
  const id = await openQuotation(page);

  // Rio Branco is credenciada with a lapsed CRF: warned, and its bid still goes in.
  const rioBranco = await agencyBids(browser, RIO_BRANCO, id, CHEAPEST_DIRECT);
  await expect(rioBranco.getByText(/CRF do FGTS vencida em/)).toBeVisible();
  await expect(rioBranco.getByText(/não pode adjudicar fornecedor com certidão vencida/)).toBeVisible();
  const amazonia = await agencyBids(browser, AMAZONIA, id, CHEAP_LONG);
  const voa = await agencyBids(browser, VOA, id, PRICIER_SHORTER);
  await expect(voa.getByText('CRF do FGTS vence em 6 dias')).toBeVisible();

  // Sealed while open: the count, never the values.
  await page.reload();
  await expect(page.getByText('3', { exact: true })).toBeVisible();
  await expect(page.getByText('R$ 1.450,00')).toHaveCount(0);

  closeBidding(id);
  await page.reload();
  await expect(page.getByText('Propostas — menor para maior')).toBeVisible();
  const row = (name: string) => page.locator('tr', { hasText: name });
  await expect(row('Rio Branco')).toContainText('1º Rio Branco Turismo LTDA');
  await expect(row('Rio Branco')).toContainText(/✘ CRF do FGTS vencida em/);
  await expect(page.getByRole('radio', { name: 'Selecionar Rio Branco Turismo LTDA' })).toBeDisabled();
  // The lowest ELIGIBLE price comes pre-selected, justified by price alone.
  await expect(page.getByRole('radio', { name: 'Selecionar Amazônia Viagens LTDA' })).toBeChecked();
  await expect(row('Amazônia')).toContainText('mais barata');
  await expect(row('Amazônia')).toContainText('8h50');
  await expect(row('Voa Roraima')).toContainText('mais rápida');
  await expect(row('Voa Roraima')).toContainText('5h40');
  await expect(row('Voa Roraima')).toContainText('CRF do FGTS vence em 6 dias');
  await expect(page.locator('#justification')).toHaveValue('Menor preço entre as propostas habilitadas.');
  await expect(page.getByRole('combobox', { name: 'Motivo da escolha' })).toHaveCount(0);

  // The pricier, shorter trip: the cost is spelled out and a real reason is required.
  await page.getByRole('radio', { name: 'Selecionar Voa Roraima Turismo LTDA' }).check();
  await expect(page.getByText('R$ 24,00 (+1,6%) acima da mais barata · 3h10 a menos de viagem.')).toBeVisible();
  const reason = page.getByRole('combobox', { name: 'Motivo da escolha' });
  await expect(reason).toContainText('Viagem mais curta que a da proposta de menor preço');
  // Only claims that hold against the cheapest are offered: same connections, so no
  // "menos conexões", and never "menor preço".
  await reason.click();
  await expect(page.getByRole('option')).toHaveText([
    /Viagem mais curta que a da proposta de menor preço/,
    /Horário compatível com o compromisso/,
    /Outro motivo/,
  ]);
  await page.keyboard.press('Escape');
  const award = page.getByRole('button', { name: 'Declarar vencedora e emitir OS' });
  await expect(page.locator('#justification')).toHaveValue('');
  await expect(award).toBeDisabled();
  await page.fill('#justification', 'Mais rápido');
  await expect(award).toBeDisabled();
  await page.fill('#justification', 'Chega 3h10 antes: a servidora tem audiência às 14h em Brasília.');
  await expect(award).toBeEnabled();
  await award.click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('Motivo: Viagem mais curta que a da proposta de menor preço');
  await expect(dialog).toContainText('3h10 a menos de viagem');
  await dialog.getByRole('button', { name: 'Confirmar adjudicação' }).click();
  await expect(page.getByText('Aguardando e-ticket da vencedora')).toBeVisible();
  await expect(page.getByText('Motivo: Viagem mais curta que a da proposta de menor preço')).toBeVisible();
  await expect(page.getByText('Chega 3h10 antes: a servidora tem audiência às 14h em Brasília.')).toBeVisible();

  // The cheaper agency is told plainly; the winner issues the ticket.
  await amazonia.reload();
  await expect(amazonia.getByText('A cotação foi adjudicada a outra proposta')).toBeVisible();
  await voa.reload();
  await expect(voa.getByText('Sua proposta venceu')).toBeVisible();
  await voa.fill('#ticketPrice', '1.523,00');
  await voa.setInputFiles('#ticketFile', {
    name: 'eticket.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('%PDF-1.4 e2e ticket'),
  });
  await voa.getByRole('button', { name: 'Anexar e-ticket' }).click();
  await expect(voa.getByText('E-ticket enviado sem divergências.')).toBeVisible();

  await page.reload();
  await expect(page.getByText('✔ Sem divergências detectadas')).toBeVisible();
  await page.getByRole('button', { name: 'Confirmar e concluir' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Concluir cotação' }).click();
  await expect(page.getByText('Concluída', { exact: true })).toBeVisible();
  expect(JSON.parse((await apiAs(page, '/audit/verify')).body).ok).toBe(true);

  // The dossiê attached to SEI: process, each bid's itinerary and eligibility, the decision.
  const token = await page.evaluate(() => localStorage.getItem('tj_token'));
  await page.goto(`${API}/quotations/${id}/report?token=${token}`);
  const report = page.locator('body');
  await expect(report).toContainText(`Processo SEI ${SEI}`);
  await expect(report).toContainText('3ª colocada — Voa Roraima Turismo LTDA — R$ 1.523,00');
  await expect(report).toContainText('Viagem mais curta que a da proposta de menor preço');
  await expect(report).toContainText(
    'R$ 24,00 (+1,6%) acima da proposta de menor preço habilitada (Amazônia Viagens LTDA, ' +
      'R$ 1.499,00); viagem 3h10 mais curta (5h40 contra 8h50); mesmo número de conexões (1 conexão).',
  );
  await expect(report).toContainText(
    /1º Rio Branco Turismo LTDA \(R\$ 1\.450,00\) — CRF do FGTS vencida em \d{2}\/\d{2}\/\d{4}/,
  );
  await expect(report).toContainText('GOL G3-1601');
  await expect(report).toContainText('Certidões da vencedora no momento da adjudicação');
  await expect(report).toContainText('Cadeia de hashes íntegra');

  // The service order carries the process and the contracted itinerary.
  await page.goto(`${API}/quotations/${id}/service-order?token=${token}`);
  await expect(page.locator('body')).toContainText(`Processo SEI ${SEI}`);
  await expect(page.locator('body')).toContainText(
    /LATAM LA-3341 \/ LA-3020 · saída \d{2}\/\d{2}\/\d{4} \d{2}:\d{2} · 5h40 · 1 conexão/,
  );
});

test('a dispute where only a lapsed supplier bid has nothing to award', async ({ page, browser }) => {
  await login(page, STAFF);
  const id = await openQuotation(page);
  await agencyBids(browser, RIO_BRANCO, id, CHEAPEST_DIRECT);
  closeBidding(id);
  await page.reload();
  await expect(page.getByText('Nenhuma proposta habilitada')).toBeVisible();
  await expect(page.getByRole('radio', { name: 'Selecionar Rio Branco Turismo LTDA' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Declarar vencedora e emitir OS' })).toHaveCount(0);
});
