import { expect, test } from '@playwright/test';
import { AMAZONIA, SEI, STAFF, apiAs, fillDemand, login, openQuotation, phone, reseed } from './support';

test.beforeEach(() => reseed());

test('the demand records its SEI process, shown to staff and never to agencies', async ({ page, browser }) => {
  await login(page, STAFF);
  await fillDemand(page, 'SEI abc');
  await page.getByRole('button', { name: 'Criar rascunho' }).click();
  await expect(page.getByText('Número do processo SEI inválido')).toBeVisible();
  await expect(page).toHaveURL(/\/cotacoes\/nova$/);

  const id = await openQuotation(page);
  await expect(page.getByText(`Processo SEI: ${SEI}`)).toBeVisible();

  const agency = await (await phone(browser)).newPage();
  await login(agency, AMAZONIA);
  await agency.goto(`/fornecedor/cotacoes/${id}`);
  await expect(agency.getByText('Tempo restante para propostas')).toBeVisible();
  await expect(agency.locator('body')).not.toContainText(SEI);
  // Not on screen, and not in what the API sends the agency either.
  const raw = await apiAs(agency, `/quotations/${id}`);
  expect(raw.status).toBe(200);
  expect(raw.body).not.toContain('seiProcess');
  expect(raw.body).not.toContain(SEI);
});
