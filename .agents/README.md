# Coordenação entre agentes (Neothasia)

Dois agentes trabalham neste repositório e comunicam **só por ficheiros nesta pasta**.

| Agente | Quem | Área (ficheiros que pode editar) |
|---|---|---|
| **A** | Claude Code (Claude desktop) | `neothesia/src/scene/menu_scene/{mod.rs,neo_btn.rs,library_page.rs,state.rs}`, `neothesia/src/library.rs`, `neothesia/src/theme.rs`, `neothesia/src/utils/capture.rs`, `neothesia/src/main.rs`, `wgpu-jumpstart/**`, `PLANO.md` |
| **B** | Agente no ZCode | `neothesia/src/scene/menu_scene/{settings.rs,tracks.rs}`, `neothesia/src/scene/playing_scene/top_bar/**`, `neothesia/src/locals.rs`, `nuon/src/settings.rs` |

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
5. **Git**: cada agente tem o seu branch. O A trabalha em `feat/ui-redesign` e o B em `feat/ui-redesign-b`, criado a partir de `feat/ui-redesign`. Não faças push para o branch do outro. Quando o B escrever "pronto para merge", o A faz o merge.
6. **Compila antes de dizer "feito"**: `cargo check -p neothesia` tem de passar sem erros.
7. **Usa o tema**: cores, medidas e animações vêm de `neothesia/src/theme.rs` (`theme::accent()`, `theme::panel(...)`, `theme::anim(...)`). Não metas cores soltas no código.
8. **Ver o resultado sem abrir a janela** com o modo de captura (`neothesia/src/utils/capture.rs`):
   ```bash
   NEOTHESIA_CAPTURE=/tmp/settings.png NEOTHESIA_PAGE=settings ./target/release/neothesia
   ```
9. Só o A reinstala a app em `/Applications/Neothesia.app`, porque o utilizador pediu isso depois de cada alteração.
10. Escreve em português de Portugal.
