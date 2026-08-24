# Roteiro do Vídeo Pitch — TJ-Viagens · máx. 5:00

> Regra de ouro (E2C4): protagonista é o PROTÓTIPO. Nada de currículos, nada de slides narrados
> sem interface. Gravar a demo com `PROPOSAL_WINDOW_MINUTES=2` e dados do seed.

## Ato I — O Gancho (0:00–1:00) · Canvas: Por quê? + Para quem?
NARRAÇÃO: Comece pela dor, não pela tecnologia. "Hoje, cada passagem aérea do TJRR nasce de um
contrato com um único fornecedor. O fiscal da SGA abre buscadores, compara telas, preenche
planilha — e ainda assim paga acima do balcão. E se precisar auditar? E-mails e planilhas
dispersas." Cite art. 79 da Lei 14.133/2021 (credenciamento p/ mercados fluidos).
TELA: foto/ilustração rápida do fluxo atual (planilha + abas) → corta para a logo TJ-Viagens.

## Ato II — A Proposta (1:00–2:00) · Canvas: O que é? · **E2C3 peso 3.0**
NARRAÇÃO: "TJ-Viagens: credenciamento permanente + disputa cega de 1 hora com preço de
referência sigiloso. Na Etapa 1 isso era um conceito; hoje é um sistema funcional: Rust,
PostgreSQL, tempo real, trilha de auditoria com hash encadeado — pronto para o container da
STI." Explicitar a EVOLUÇÃO: conceito → regras do edital implementadas uma a uma.
TELA: diagrama simples dos 3 módulos (credenciamento → disputa → auditoria) e a tabela de
regras do edital com ✔.

## Ato III — O Motor sob o Capô (2:00–3:30) · Canvas: Funcionamento + Características · **E2C1/E2C2 peso 2.5+2.5 — SHOW, DON'T TELL**
DEMO AO VIVO (roteiro do README, ensaiado; janela A servidor, janela B fornecedor em largura de
celular):
- 2:00 Servidor cria demanda (form do passageiro) e ABRE → diálogo "notificará todos os ativos".
- 2:20 Fornecedor no celular: notificação → cronômetro gigante → envia 1.523,00. Segundo
  fornecedor envia 1.499,00. Narrar: "cada um vê só a própria proposta — sigilo e isonomia".
- 2:45 Painel do servidor: contagem sobe ao vivo, valores lacrados. Janela encerra sozinha
  (relógio do servidor).
- 2:55 Ranking menor→maior com Δ vs referência; 1ª pré-selecionada → 1 clique "Declarar
  vencedora e emitir OS".
- 3:10 Fornecedora vencedora: banner 🏆, prazo de 30 min, anexa e-ticket → conferência
  automática sem divergências → servidor confirma.
- 3:25 Mostrar OS imprimível e o selo "trilha de auditoria íntegra" (mencionar SEI/integração).

## Ato IV — O Impacto (3:30–4:30) · Canvas: Resultados esperados
NARRAÇÃO: converter em valor institucional: "Nesta cotação, R$ 351,00 abaixo da referência —
18,97%. No painel acumulado: R$ 702,00 economizados em 2 cotações adjudicadas, média de 2,5
participantes por disputa, 100% dos e-tickets anexados no prazo — os indicadores do próprio
edital, medidos pelo sistema, não por planilha." Horas → minutos por cotação; auditoria deixa de
ser reconstrução manual.
TELA: dashboard com KPIs + relatório de economicidade da cotação semeada.

## Ato V — O Fechamento (4:30–5:00) · Síntese · E2C4 peso 2.0
NARRAÇÃO: "Código aberto, PostgreSQL, sem APIs pagas, sem raspagem, LGPD desde a concepção —
pronto para o sandbox da Etapa 3 com a STI. A Entropy Code entrega hoje o que o desafio pediu:
menor preço, celeridade e rastreabilidade total." Encerrar com nome da equipe + TJ-Viagens.
TELA: tela final com logo, stack e QR/URL do repositório.
