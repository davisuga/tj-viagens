import { expect, test } from '@playwright/test';
import { RIO_BRANCO, STAFF, VOA, login, phone, reseed } from './support';

test.beforeEach(() => reseed());

test('the servidor sees credenciado apart from certidões em dia', async ({ page }) => {
  await login(page, STAFF);
  await page.goto('/fornecedores');
  const row = (name: string) => page.locator('tr', { hasText: name });

  // Credenciada months ago, CRF lapsed since: still listed as credenciada.
  await expect(row('Rio Branco')).toContainText(/✘ CRF do FGTS vencida em \d{2}\/\d{2}\/\d{4}/);
  await expect(row('Rio Branco')).toContainText('Credenciado');
  await expect(row('Voa Roraima')).toContainText('⚠ CRF do FGTS vence em 6 dias');
  await expect(row('Amazônia')).toContainText('✔ em dia');
  await expect(row('Monte Roraima')).toContainText('✘ CNDT ausente');
  await expect(row('Monte Roraima').getByRole('button', { name: 'Homologar' })).toBeDisabled();
});

test('each agency sees its certidões and what a lapse costs', async ({ browser }) => {
  const voa = await (await phone(browser)).newPage();
  await login(voa, VOA);
  await expect(voa.getByText(/⚠ vence em 6 dias \(\d{2}\/\d{2}\/\d{4}\)/)).toBeVisible();
  await expect(voa.getByText('sua proposta não pode ser adjudicada')).toHaveCount(0);

  const rioBranco = await (await phone(browser)).newPage();
  await login(rioBranco, RIO_BRANCO);
  await expect(rioBranco.getByText(/⚠ vencida em \d{2}\/\d{2}\/\d{4} — reenvie/)).toBeVisible();
  await expect(rioBranco.getByText('sua proposta não pode ser adjudicada')).toBeVisible();
});
