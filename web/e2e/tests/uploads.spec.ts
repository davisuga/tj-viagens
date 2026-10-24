import { expect, test } from '@playwright/test';
import { VOA, login, phone, reseed } from './support';

test.beforeEach(() => reseed());

/** A PDF-looking file of the given size: real scans run to several MB, which the
 *  tiny fixtures elsewhere in the suite never exercise. */
function pdfOf(megabytes: number, name: string) {
  const buffer = Buffer.alloc(Math.round(megabytes * 1024 * 1024), 0x20);
  buffer.write('%PDF-1.4\n');
  return { name, mimeType: 'application/pdf', buffer };
}

test('an agency uploads a real-sized scan, and an oversized file is refused plainly', async ({ browser }) => {
  const agency = await (await phone(browser)).newPage();
  await login(agency, VOA);
  await expect(agency.getByText('PDF ou imagem, até 10 MB.')).toBeVisible();

  // A 3 MB scanned contrato social goes through.
  await agency.setInputFiles('#docFile', pdfOf(3, 'contrato-social-escaneado.pdf'));
  await agency.getByRole('button', { name: 'Enviar documento' }).click();
  await expect(agency.getByText('Documento enviado.')).toBeVisible();

  // Over the limit: stopped before upload, with the limit stated.
  await agency.setInputFiles('#docFile', pdfOf(11, 'scan-enorme.pdf'));
  await agency.getByRole('button', { name: 'Enviar documento' }).click();
  await expect(agency.getByText('Arquivo grande demais — o limite é 10 MB.')).toBeVisible();
  await expect(agency.getByText('Falha de comunicação com o servidor.')).toHaveCount(0);
});
