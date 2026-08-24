# Canvas do Protótipo — TJ-Viagens (Entropy Code)

## O que é?
Plataforma web funcional (não mockup) de credenciamento permanente e cotação competitiva de
passagens aéreas para o TJRR. O público interage com dois ambientes: o painel do servidor da SGA
(demanda, disputa, adjudicação, conferência, dossiê) e o portal do fornecedor credenciado
(documentos, proposta sigilosa com cronômetro, e-ticket).

## Por quê?
Hoje a aquisição depende de contrato com fornecedor único: tarifas acima do balcão, fiscais
comparando telas e planilhas manualmente, histórico disperso e difícil de auditar (dores do
Edital 13/2026, Desafio 1). O art. 79 da Lei 14.133/2021 autoriza credenciamento para mercados
fluidos — faltava a ferramenta.

## Para quem?
- Servidores da SGA/fiscais (registram demanda, declaram vencedora, conferem bilhete)
- Agências e companhias credenciadas (disputam e emitem)
- Gestão e controle interno (KPIs, dossiê pronto para o SEI, trilha auditável)

## Resultados esperados
- Economia mensurável por cotação (referência sigilosa × preço contratado) e acumulada
- Esforço do servidor reduzido de horas para minutos por cotação
- 100% das disputas com notificação simultânea, janela isonômica de 1h e ranking automático
- E-ticket vinculado em até 30 min (medido) e rastreabilidade integral verificável por hash

## Funcionamento
1. Credenciamento aberto: cadastro com validação de CNPJ + documentos fiscais/trabalhistas com
   checklist determinístico; homologação humana.
2. Servidor registra a demanda (passageiro Nome/CPF/Sexo/Nascimento, trecho, voo e preço de
   referência sigiloso) e abre a disputa: todos os ativos notificados, cronômetro de 1h no
   relógio do servidor.
3. Propostas cegas (valor + voo), substituíveis até o fim; ninguém vê referência nem rivais.
4. Encerrada: tabela menor→maior, 1ª pré-selecionada, declaração em 1 clique, OS emitida.
5. Vencedora anexa e-ticket em 30 min; conferência automática aponta divergências; servidor
   confirma; relatório de economicidade e dossiê imprimível são gerados.

## Características
- Rust + Axum + PostgreSQL, SSE em tempo real, código aberto e conteinerizável (STI)
- Trilha append-only com encadeamento SHA-256 e endpoint de verificação de integridade
- LGPD: preço de referência e PII segregados por papel; CPF mascarado em relatórios
- Regras determinísticas no lugar de IA obrigatória (pré-triagem documental e conferência de
  bilhete) — sem custo por requisição, sem dependência externa
- UI Fluid Functionalism responsiva: fila de ações do servidor, disputa mobile-first
