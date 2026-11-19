# Quadro de tarefas

Estados possíveis: `por fazer`, `em curso`, `bloqueada`, `feito`.
Para pegar numa tarefa, muda o estado e escreve uma linha em `MENSAGENS.md`.

| ID | Dono | Tarefa | Estado | Feito quando… |
|---|---|---|---|---|
| A1 | A | Tema partilhado `theme.rs` | feito | Existe e compila |
| A2 | A | Modo de captura PNG a funcionar | em curso | `NEOTHESIA_CAPTURE=… ./target/release/neothesia` grava um PNG |
| A3 | A | Componentes base: botão, botão-ícone, campo de pesquisa, linha de lista (em `neo_btn.rs`/novo `widgets.rs`) | por fazer | Usados no menu e na biblioteca |
| A4 | A | Menu principal novo (Fase 1) | por fazer | Cartão Continuar, ações em cartões, Recentes |
| A5 | A | Biblioteca nova (Fase 2) | por fazer | Barra lateral, pesquisa, teclado, fila melhorada |
| A6 | A | API da playlist para o B: `Library::previous()`, `Library::position_label()` | feito | Métodos em `library.rs`, avisar o B |
| B1 | B | Redesenhar Definições (`settings.rs`, `nuon/src/settings.rs`) com `theme.rs` | por fazer | Captura `NEOTHESIA_PAGE=settings` com o novo visual, `cargo check` OK |
| B2 | B | Redesenhar Faixas (`tracks.rs`) com `theme.rs` | por fazer | Captura `NEOTHESIA_PAGE=tracks` (com uma música aberta) com o novo visual |
| B3 | B | Barra de cima do ecrã de tocar: título, "3 de 8", ⏮ ⏭ | por fazer | Botões funcionam com uma playlist de 2+ músicas |
| B4 | B | Traduzir `locals.rs` para PT-PT (acrescentar ramo `"pt"`) | por fazer | Com o sistema em português, todos os textos aparecem em PT |

## Notas para o B

- Começa por **B4** e **B1**, que não dependem de nada.
- Lê `neothesia/src/theme.rs` antes de desenhar. O estilo de referência é `library_page.rs` (painéis, linhas e hover).
- Cria o teu branch: `git checkout feat/ui-redesign && git checkout -b feat/ui-redesign-b`.
