import { expect, test } from '@playwright/test';
import { DEPARTURE_DAY, STAFF, VOA, login, openQuotation, phone, reseed } from './support';

test.beforeEach(() => reseed());

test('the bid form fits a phone and starts from the requested departure', async ({ page, browser }) => {
  await login(page, STAFF);
  const id = await openQuotation(page);

  const agency = await (await phone(browser)).newPage();
  await login(agency, VOA);
  await agency.goto(`/fornecedor/cotacoes/${id}`);
  await expect(agency.getByText('Tempo restante para propostas')).toBeVisible();
  // The agency only adjusts the time of the requested departure.
  await expect(agency.locator('#departsAt')).toHaveValue(`${DEPARTURE_DAY}T08:00`);
  await agency.fill('#flight', 'LA-3341 / LA-3020');
  await agency.fill('#duration', '5:40');
  await expect(agency.getByText('= 5h40, da saída à chegada, contando conexões')).toBeVisible();

  // Nothing scrolls sideways or gets clipped at 390px.
  expect(await agency.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
  const price = await agency.locator('#price').boundingBox();
  const connections = await agency.getByRole('combobox', { name: 'Conexões' }).boundingBox();
  if (!price || !connections) throw new Error('bid form not rendered');
  expect(connections.x + connections.width).toBeLessThanOrEqual(price.x + price.width + 1);
  expect(
    await agency.locator('#flight').evaluate((el) => el.scrollWidth <= el.clientWidth),
  ).toBe(true);
  expect((await agency.locator('#departsAt').boundingBox())?.width ?? 0).toBeGreaterThan(250);
});
