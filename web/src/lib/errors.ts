const MESSAGES: Record<string, string> = {
  NAO_AUTENTICADO: 'Sessão expirada — entre novamente.',
  ACESSO_NEGADO: 'Você não tem permissão para esta ação.',
  CNPJ_INVALIDO: 'CNPJ inválido — confira os dígitos.',
  JA_CADASTRADO: 'CNPJ ou e-mail já cadastrado.',
  SENHA_CURTA: 'A senha precisa de pelo menos 8 caracteres.',
  SENHA_LONGA: 'Senha longa demais.',
  DOCUMENTO_INVALIDO: 'Documento inválido — selecione o tipo e o arquivo.',
  CHECKLIST_PENDENTE: 'Checklist incompleto — documentos faltando ou vencidos.',
  JA_DECIDIDO: 'Este credenciamento já foi decidido.',
  DECISAO_INVALIDA: 'Decisão inválida.',
  FORNECEDOR_NAO_ATIVO: 'Seu credenciamento ainda não está ativo.',
  NAO_ENCONTRADA: 'Cotação não encontrada.',
  NAO_ENCONTRADO: 'Registro não encontrado.',
  NAO_ESTA_EM_RASCUNHO: 'A cotação não está mais em rascunho.',
  CPF_INVALIDO: 'CPF inválido.',
  SEXO_INVALIDO: 'Valor de sexo inválido.',
  PRECO_INVALIDO: 'Valor inválido — use o formato 1.234,56.',
  VOO_INVALIDO: 'Informe o voo ofertado (até 200 caracteres).',
  OBSERVACOES_LONGAS: 'Observações longas demais (máx. 2000 caracteres).',
  COTACAO_FECHADA: 'A janela de propostas já encerrou.',
  COTACAO_AINDA_ABERTA: 'A cotação ainda está aberta.',
  NAO_ESTA_FECHADA: 'A cotação não está encerrada (ou já foi adjudicada).',
  PROPOSTA_INVALIDA: 'Proposta inválida para esta cotação.',
  JUSTIFICATIVA_CURTA: 'Escreva uma justificativa (mín. 5 caracteres).',
  NAO_AGUARDA_BILHETE: 'Esta cotação não aguarda e-ticket.',
  BILHETE_INVALIDO: 'Dados do bilhete inválidos.',
  BILHETE_NAO_ENVIADO: 'Nenhum e-ticket enviado.',
  STATUS_INVALIDO: 'Status não permite esta ação.',
  OS_NAO_EMITIDA: 'Ordem de Serviço ainda não emitida.',
  ERRO_INTERNO: 'Erro interno — tente novamente.',
};

export function errorMessage(err: unknown): string {
  if (err instanceof Error) {
    return (
      MESSAGES[err.message] ??
      (err.message.startsWith('HTTP') ? 'Falha de comunicação com o servidor.' : err.message)
    );
  }
  return 'Falha inesperada.';
}
