import { expect, test } from '@playwright/test';
import {
  AMAZONIA, API, CHEAP_LONG, PRICIER_SHORTER, STAFF, VOA, agencyBids, closeBidding, login,
  openQuotation, reseed,
} from './support';

test.beforeEach(() => reseed());

test('awarding the cheapest eligible bid stays a one-click decision on price', async ({ page, browser }) => {
  await login(page, STAFF);
  const id = await openQuotation(page);
  await agencyBids(browser, AMAZONIA, id, CHEAP_LONG);
  await agencyBids(browser, VOA, id, PRICIER_SHORTER);
  closeBidding(id);
  await page.reload();

  const cheapest = page.getByRole('radio', { name: 'Selecionar Amazônia Viagens LTDA' });
  const shorter = page.getByRole('radio', { name: 'Selecionar Voa Roraima Turismo LTDA' });
  const justification = page.locator('#justification');
  await expect(cheapest).toBeChecked();
  await expect(page.getByText('Proposta mais cara')).toHaveCount(0);

  // Trying the other bid and coming back restores the price justification, so a
  // stale "menor preço" text can never ride along with a pricier pick.
  await shorter.check();
  await expect(justification).toHaveValue('');
  await cheapest.check();
  await expect(justification).toHaveValue('Menor preço entre as propostas habilitadas.');
  await expect(page.getByRole('combobox', { name: 'Motivo da escolha' })).toHaveCount(0);

  await page.getByRole('button', { name: 'Declarar vencedora e emitir OS' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('Motivo: Menor preço entre as propostas habilitadas');
  await dialog.getByRole('button', { name: 'Confirmar adjudicação' }).click();
  await expect(page.getByText('Aguardando e-ticket da vencedora')).toBeVisible();

  const token = await page.evaluate(() => localStorage.getItem('tj_token'));
  await page.goto(`${API}/quotations/${id}/report?token=${token}`);
  const report = page.locator('body');
  await expect(report).toContainText('1ª colocada — Amazônia Viagens LTDA — R$ 1.499,00');
  await expect(report).toContainText('Menor preço entre as propostas habilitadas');
  await expect(report).not.toContainText('Comparativo');
  await expect(report).not.toContainText('Desconsiderada');
});
