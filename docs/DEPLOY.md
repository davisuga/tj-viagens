# Deploy

A instância de demonstração roda em **https://sh-tjviagens-74-208-159-201.sslip.io**,
num VPS gerenciado por [Dokploy](https://dokploy.com), atrás do Traefik (TLS via
Let's Encrypt).

> Nenhum segredo mora neste repositório. Chave de API do Dokploy, acesso SSH e as
> senhas geradas do stack ficam no runbook privado `~/ops-chief-deploy-runbook.md`
> (fora de qualquer repositório, modo `600`).

## Desenho

```
workstation ── cargo zigbuild (amd64) ──┐
            ── bun run build            ├─▶ docker save │ ssh docker load ─▶ VPS
                                        ┘                                     │
                                                                 Traefik ─▶ web (nginx)
                                                                              ├─ /        SPA
                                                                              └─ /api/ ─▶ api (Rust) ─▶ db
```

Três decisões que sustentam o resto:

- **Nada compila no VPS.** Ele serve tráfego; uma build Rust o deixaria de joelhos.
  O binário é cross-compilado com `cargo-zigbuild` (piso de glibc 2.28, roda em
  `bookworm`) e a imagem viaja pela própria conexão SSH — não há registry envolvido.
- **Origem única.** O nginx serve o bundle estático e faz proxy de `/api/` para o
  serviço Rust removendo o prefixo, então `VITE_API_URL` é o relativo `/api`: o
  bundle não carrega hostname nenhum e não existe CORS entre front e back.
- **SSE precisa de `proxy_buffering off`.** Sem isso o fluxo de eventos da cotação
  fica preso no buffer do nginx e o cronômetro do fornecedor congela.

## Pré-requisitos

`docker` + `buildx`, `bun`, `zig` e `cargo-zigbuild`, `jq`, e uma conexão SSH com o
host. O painel do Dokploy escuta apenas em `127.0.0.1:3000` (a porta não é pública),
então o mais simples é abrir um ControlMaster que já encaminha a porta:

```bash
ssh -M -S /tmp/tjv-cm -o ControlPersist=6h -L 3000:127.0.0.1:3000 -N -f root@$VPS
```

Tudo depois disso multiplexa nessa conexão, sem senha e sem alterar nada no servidor.

## Publicar uma versão

```bash
DOKPLOY_API_KEY=... ./scripts/deploy.sh
```

O script cross-compila, monta as duas imagens, envia para o host, aponta a variável
`TAG` para o commit atual, dispara o deploy e espera `/api/health` responder.

## Semear os dados de demonstração

Recria a base do zero — **apaga tudo** que estiver lá.

```bash
ssh -S /tmp/tjv-cm root@$VPS 'docker exec tjviagens-soxeyp-api-1 seed'
```

## Janelas de tempo

O ambiente publicado usa as regras reais: proposta em 60 min, e-ticket em 30 min. A
cotação semeada já está concluída, então o fluxo inteiro é visível sem esperar. Para
uma demonstração ao vivo do ciclo completo, encurte pelo painel do Dokploy
(`PROPOSAL_WINDOW_MINUTES`, `TICKET_WINDOW_MINUTES`) e redeploy — a interface não
menciona duração fixa em lugar nenhum.

## Armadilhas conhecidas

- **O store de env do Dokploy substitui, não mescla.** Leia com `compose.one`, edite
  a linha, devolva inteiro. Mandar só a variável nova apaga as demais.
- **`compose.update` é patch parcial** — um campo por chamada (`composeFile` *ou*
  `env`), nunca o objeto todo.
- **`sourceType` nasce como `github`.** Um compose criado pela API falha com
  `Github Provider not found` até receber `{"sourceType":"raw"}`.
- **Arquitetura.** O VPS é amd64; imagem de Apple Silicon entra em crash-loop com
  `exec format error`.
- **`POSTGRES_PASSWORD` só vale na primeira inicialização do volume.** Rotacionar
  exige `ALTER USER` no banco vivo, além da variável.
