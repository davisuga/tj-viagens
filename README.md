# TJ-Viagens — Cotações competitivas de passagens aéreas (TJRR)

Protótipo (Etapa 2, Prêmio de Inovação TJRR — Edital 13/2026, Tema 1 / Desafio 1) da equipe
**Entropy Code**. Credenciamento permanente de agências/companhias + disputa cega com preço de
referência sigiloso, janela de 1 hora controlada pelo relógio do servidor, seleção pelo menor
preço, Ordem de Serviço automática, e-ticket em 30 minutos e trilha de auditoria com
encadeamento de hashes.

## Ambiente publicado

**https://sh-tjviagens-74-208-159-201.sslip.io** — instância de demonstração com os dados
semeados (credenciais abaixo). Detalhes de operação em [docs/DEPLOY.md](docs/DEPLOY.md).

## Stack

- **API**: Rust · Axum · SQLx · PostgreSQL 16 · SSE · askama (OS e relatório imprimíveis)
- **Web**: Bun · Vite · React · Tailwind 4 · shadcn/ui + Fluid Functionalism · TanStack Query
- Sem APIs comerciais de voo, sem scraping, sem IA obrigatória (pré-triagem e conferência são
  regras determinísticas), conforme restrições do desafio.

## Subir do zero

```bash
docker compose up -d              # postgres :5433 (dev + test)
cd api && cargo run --bin seed    # migra + dados fictícios de demonstração
cargo run --bin api               # http://localhost:3001
# noutro terminal:
cd web && bun install && bun run dev   # http://localhost:5173
```

Testes: `cd api && cargo test -- --test-threads=1` (usa o banco `tjviagens_test`).

## Credenciais de demonstração (senha `demo1234`)

| Perfil | E-mail |
|---|---|
| Servidor SGA | servidor@tjrr.jus.br |
| Admin | admin@tjrr.jus.br |
| Fornecedor (ativo) | contato@voaroraima.com.br |
| Fornecedor (ativo) | contato@amazoniaviagens.com.br |
| Fornecedor (ativo) | contato@riobrancotur.com.br |
| Fornecedor (pendente, falta CNDT) | contato@monteroraima.com.br |

## Roteiro de demonstração (~90s, espelha o Ato III do pitch)

Para demo ao vivo, encurte as janelas em `api/.env`:
`PROPOSAL_WINDOW_MINUTES=2` e `TICKET_WINDOW_MINUTES=5` (reinicie a API).

1. **Servidor** (janela A, `servidor@tjrr.jus.br`): dashboard já mostra economia acumulada da
   cotação semeada. "Nova cotação" → dados do passageiro → criar → **Abrir** (diálogo avisa a
   notificação simultânea).
2. **Fornecedor** (janela B em largura de celular, `contato@voaroraima.com.br`): notificação no
   painel → cotação → cronômetro grande → envia proposta (ex.: `1.523,00`). Repita com
   `contato@amazoniaviagens.com.br` com preço menor (ex.: `1.499,00`).
3. Janela A: contagem de propostas sobe ao vivo (valores lacrados). Ao encerrar: ranking
   menor→maior com a 1ª pré-selecionada → **Declarar vencedora e emitir OS** (1 clique).
4. Janela B (vencedora): banner 🏆 + prazo de 30 min → anexa e-ticket.
5. Janela A: conferência lado a lado sem divergências → **Confirmar** → card de economia,
   selo "trilha de auditoria íntegra", botões OS/Relatório imprimíveis (anexáveis ao SEI).

## Estrutura

- `api/` — crate Rust (rotas em `src/routes/`, domínio puro em `src/domain/`, auditoria em
  `src/audit.rs`, templates imprimíveis em `templates/`)
- `web/` — SPA React (área do servidor e do fornecedor)
- `docs/pitch/` — canvas, roteiro do vídeo (≤5 min) e checklist de gravação da Etapa 2
- `docs/superpowers/plans/` — plano de implementação executável
