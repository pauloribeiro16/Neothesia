# Coordenação entre agentes (Neothasia)

Dois agentes trabalham neste repositório e comunicam **só por ficheiros nesta pasta**. O A é o arquiteto e define *como*; o B implementa. Lê `../ARQUITETURA.md` antes de escrever código.

| Agente | Papel | Ficheiros que pode editar |
|---|---|---|
| **A** (Claude Code) | **Arquiteto**: estrutura, componentes, tema, especificações, revisão, merge e instalação | `ARQUITETURA.md`, `PLANO.md`, `.agents/TAREFAS.md`, `neothesia/src/{main.rs,context.rs,library.rs,theme.rs,widgets.rs,utils/**}`, `neothesia/src/scene/menu_scene/{mod.rs,state.rs}`, `neothesia/src/scene/playing_scene/mod.rs`, crates de infraestrutura (`nuon/`, `neothesia-core/`, `wgpu-jumpstart/`, …) |
| **B** (ZCode) | **Implementação** dos ecrãs, seguindo `ARQUITETURA.md` e as especificações em `TAREFAS.md` | `neothesia/src/scene/menu_scene/{main_page.rs,library_page.rs,settings.rs,tracks.rs,neo_btn.rs}`, `neothesia/src/scene/playing_scene/top_bar/**`, `neothesia/src/locals.rs` |

Para editar ficheiros fora da tua lista, pede primeiro em `MENSAGENS.md` e espera resposta.

## Ficheiros

- `../PLANO.md`: plano geral e fases. É a fonte de verdade sobre *o quê* e *porquê*.
- `TAREFAS.md`: quadro de tarefas, cada uma com dono (A/B), estado e critério de "feito".
- `MENSAGENS.md`: caixa de correio. Acrescenta sempre no **fim** e nunca apagues mensagens.

## Regras

1. **Antes de começar**, lê `PLANO.md`, `TAREFAS.md` e `MENSAGENS.md`.
2. Para pegar numa tarefa, muda o estado em `TAREFAS.md` para `em curso`.
3. Ao terminar, muda o estado para `feito` e escreve em `MENSAGENS.md` o que mudou e que ficheiros tocaste.
4. Formato das mensagens:
   ```
   ## AAAA-MM-DD HH:MM · A → B · assunto
   Texto curto: o quê, ficheiros, o que falta, dúvidas.
   ```
5. **Git e pastas.** Cada agente trabalha na **sua própria pasta**, para nunca mexer nos ficheiros do outro:
   - A: `~/Desktop/Projetos/Neothasia`, branch `feat/ui-redesign`
   - B: `~/Desktop/Projetos/Neothasia-B` (git worktree, já criada), branch `feat/ui-redesign-b`

   Os ficheiros de coordenação vivem **só** em `~/Desktop/Projetos/Neothasia/.agents/`, e o B lê e escreve nesse caminho absoluto. O B faz commits no seu branch e pode fazer push para o remote `fork`. Para trazer o trabalho do A, corre `git merge feat/ui-redesign`. Quando o B escrever "pronto para merge", o A faz o merge.
6. **Compila antes de dizer "feito"**: `cargo check -p neothesia` tem de passar sem erros.
7. **Usa o tema**: cores, medidas e animações vêm de `neothesia/src/theme.rs` (`theme::accent()`, `theme::panel(...)`, `theme::anim(...)`). Não metas cores soltas no código.
8. **Ver o resultado sem abrir a janela** com o modo de captura (`neothesia/src/utils/capture.rs`):
   ```bash
   NEOTHESIA_CAPTURE=/tmp/settings.png NEOTHESIA_PAGE=settings ./target/release/neothesia
   ```
9. Só o A reinstala a app em `/Applications/Neothesia.app`, porque o utilizador pediu isso depois de cada alteração.
10. Escreve em português de Portugal.
