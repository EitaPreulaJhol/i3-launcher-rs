# i3-launcher-rs

Launcher estilo `dmenu` para o i3: uma **strip horizontal** (topo ou base do
monitor, configurável) com os **ícones dos aplicativos**, busca incremental e
execução de entries `.desktop` — sem toolkit gráfico, só X11 + renderização por
software.

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Run: firef▏   [🦊 Firefox]  [🌐 Chromium]  [📝 Gedit]  [⌨ Alacritty]      › │
└────────────────────────────────────────────────────────────────────────────┘
```

## Recursos

- Strip `override-redirect`: o i3 não gerencia, não há bordas, gaps ou foco roubado.
- Posição configurável: topo/base, offset, margens, largura (px ou %) e alinhamento.
- Multi-monitor via RandR 1.5 (fallback Xinerama): monitor focado, primário,
  sob o mouse, por índice ou por nome do conector.
- Ícones pelo padrão XDG Icon Theme, com herança de tema, escolha de tamanho,
  PNG redimensionado com Lanczos e SVG rasterizado no tamanho final.
- Busca `prefix`, `substring` ou `fuzzy`, com score; atalhos de edição de linha
  no estilo dmenu/readline.
- Execução correta do `Exec` do Desktop Entry (quoting, field codes, `Terminal=true`
  e `Path=`), com `setsid` quando disponível.
- Modo dmenu (`--dmenu`) para usar em pipes: lê linhas do stdin e imprime a seleção.
- Config TOML com validação (`deny_unknown_fields`), flags compatíveis com dmenu.
- Início rápido: ~25 ms até o primeiro frame (release, 38 apps, fontes já em cache do SO).

## Instalação

Dependências de build: Rust estável e as bibliotecas de desenvolvimento de
X11/XCB usuais (em Debian/Ubuntu: `libxcb1-dev libxcb-randr0-dev
libxcb-xinerama0-dev`). O `cosmic-text` usa fontconfig.

```bash
git clone <este-repo> i3-launcher-rs
cd i3-launcher-rs
cargo build --release
install -Dm755 target/release/i3-launcher ~/.local/bin/i3-launcher
# ou: make install
```

No `~/.config/i3/config`:

```
bindsym $mod+d exec --no-startup-id i3-launcher
# variantes
bindsym $mod+Shift+d exec --no-startup-id i3-launcher -b      # na base
bindsym $mod+x exec --no-startup-id i3-launcher --print       # imprime a seleção
```

> Se você usa i3bar no topo, a strip cobre a barra enquanto está aberta (como o
> dmenu). Para aparecer logo abaixo dela, use `offset = 24` (ou a altura do seu
> i3bar) na seção `[bar]`.

## Uso

```
i3-launcher [OPÇÕES]

  -p, --prompt <TEXTO>       texto do prompt
  -b, --bottom               ancora na base do monitor
  -e, --edge <top|bottom>
  -m, --monitor <ALVO>       focused | primary | pointer | ÍNDICE | NOME
      --height <PX>  --margin <PX>  --offset <PX>  --width <PX|%>
      --align <left|center|right>
  -fn, --font <FAMÍLIA>      família da fonte
      --font-size <PX>       tamanho da fonte
      --icon-size <PX>       tamanho dos ícones
      --no-icons             não desenha ícones
      --terminal <CMD>       terminal para entradas Terminal=true
      --case-sensitive  --match <prefix|substring|fuzzy>
  -d, --dmenu                lê itens do stdin e imprime a seleção
      --exec                 com --dmenu, executa via shell
      --print                imprime a seleção em vez de executar
  -nb/-nf/-sb/-sf <COR>      cores (compatibilidade com dmenu)
  -c, --config <ARQUIVO>     config TOML
  -h, --help  -v, --version
```

Config: `$I3L_CONFIG`, `--config`, `$XDG_CONFIG_HOME/i3-launcher/config.toml` ou
`~/.config/i3-launcher/config.toml`. Veja `config.example.toml` para todas as
chaves.

### Teclas

| Tecla | Ação |
| --- | --- |
| `Enter` | executa o item selecionado |
| `Esc` | fecha (código 1) |
| `↑` `↓` / `Ctrl+P` `Ctrl+N` | move a seleção |
| `PgUp` `PgDn` | salta 10 itens |
| `←` `→` / `Ctrl+B` `Ctrl+F` | move o cursor |
| `Alt+B` `Alt+F` | move por palavra |
| `Home` `End` / `Ctrl+A` `Ctrl+E` | início/fim do texto |
| `Backspace` `Delete` / `Ctrl+H` `Ctrl+D` | apaga caractere |
| `Ctrl+W` `Ctrl+U` `Ctrl+K` | apaga palavra / até o início / até o fim |
| `Tab` | completa com o item selecionado |
| clique / scroll | seleciona / navega |

### Modo dmenu

```bash
printf 'um\ndois\ntres\n' | i3-launcher --dmenu --prompt 'Escolha: '
# imprime a linha escolhida em stdout

ls | i3-launcher --dmenu --exec   # executa via /bin/sh
```

### `--print`

Imprime o nome do aplicativo selecionado em vez de executá-lo. Útil para testes
e integração com scripts.

## Como funciona

```text
.desktop ──scan──▶ itens ──filtro(query)──▶ score ──layout──▶ tiny-skia ──PutImage──▶ X11
     stdin ────────┘                              ▲                                   │
                                                  └────────── teclado/mouse ───────────┘
```

- **Janela**: `x11rb`, `override-redirect`, `PutImage` em um pixmap de
  double-buffer; grab exclusivo do teclado (o mouse continua livre).
- **Texto**: `cosmic-text` (shaping, fallback de fontes) com rasterização de
  glifos e blend manual no pixmap.
- **Ícones**: resolução XDG + cache por `(nome, tamanho)`; PNG com Lanczos, SVG
  com `resvg` no tamanho alvo.
- **Desktop Entry**: parser próprio (grupos, chaves localizadas, escapes,
  `Exec` com field codes) e filtros (`Type`, `NoDisplay`, `Hidden`, `TryExec`,
  `OnlyShowIn`/`NotShowIn`).

Detalhes de arquitetura e roadmap: [`PLAN.md`](PLAN.md).

## Desenvolvimento

```bash
cargo test          # parser, scan, ícones, matching, edição, geometria
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run --release
```

Ferramentas de depuração (sem abrir janela, exceto a última):

```bash
cargo run --release --example measure       # larguras medidas pelo cosmic-text
cargo run --release --example layout_dump   # células/viewport para 1366px
cargo run --release --example match_dump -- ala
I3L_DEBUG=1 cargo run --release             # log de monitor/itens/tema
```

## Limitações conhecidas

- **X11 apenas** (i3/Xorg). Wayland/layer-shell está no roadmap.
- Sem IME nem teclas mortas: o mapa de teclado usa `GetKeyboardMapping`
  (níveis 0/1 com Shift). Para teclados complexos, o caminho é `xkbcommon`.
- Sem layout em grade: a lista é uma única linha com scroll horizontal, como o
  dmenu.
- Cantos arredondados da strip exigiriam visual ARGB + compositor; por enquanto
  só o destaque de seleção é arredondado.
- Ícones SVG com `<text>` não renderizam texto (resvg sem a feature `text`).

## Licença

MIT.
