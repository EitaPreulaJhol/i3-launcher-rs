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
- **Estilo no TOML**: seção `[style]` com cores, raios, bordas, espaçamentos,
  peso da fonte e 14 temas prontos (`--themes`).
- **Tema padrão fiel ao `original_source/dmenu.rasi`**: barra clara
  (`#DEDEDE`/`#333333`), seleção `#C5C5C5` com texto preto e borda de 1px
  `#FF8C00`, cantos retos, 20px de altura e fonte `Sans 9`.
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
      --theme <NOME>         sobrescreve o tema de cores do config
      --themes               lista os temas disponíveis
  -c, --config <ARQUIVO>     config TOML
  -h, --help  -v, --version
```

Config: `$I3L_CONFIG`, `--config`, `$XDG_CONFIG_HOME/i3-launcher/config.toml` ou
`~/.config/i3-launcher/config.toml`. Veja `config.example.toml` para todas as
chaves.

## Estilo e temas

Tudo que é visual fica em `[style]`. O padrão (sem config) reproduz o
`original_source/dmenu.rasi`, o mesmo esquema da barra do i3/polybar:

```toml
[appearance]
font = "Sans"                 # "Sans 9" no rasi (9pt ≈ 12px a 96dpi)
font_size = 12.0
icon_size = 14                # ~1.2em

[style]
theme = ""                    # vazio = padrão do dmenu.rasi
padding = 2.0                 # window { padding: 2px; }
gap = 0.0
item_gap = 0.0                # listview { spacing: 0px; }
item_padding = 10.0           # element { padding: 0px 10px; }
search_width = 120            # entry { width: 10em; }
selection_radius = 0.0        # cantos retos
selection_border_width = 1.0
selection_border = "#ff8c00"  # element selected
```

As cores do padrão são `bg = "#dedede"`, `fg = "#333333"`,
`selection_bg = "#c5c5c5"` e `selection_fg = "#000000"`; o rasi não destaca os
caracteres casados, então `highlight_fg` é opcional e vem desligado.

Qualquer cor escrita em `[style]` vence o tema. Para um visual arredondado,
caixa de entrada e borda no painel:

```toml
[style]
theme = "tokyo-night"
selection_radius = 10.0
selection_border_width = 2.0
selection_border = "#9ece6a"
input_bg = "#00000066"
input_radius = 10.0
input_border_width = 1.5
input_border = "#7aa2f7"
panel_border_width = 2.0
panel_border = "#7aa2f7"
highlight_fg = "#9ece6a"      # habilita o destaque da busca
```

Temas incluídos: `catppuccin-mocha`, `catppuccin-macchiato`,
`catppuccin-frappe`, `catppuccin-latte`, `gruvbox-dark`, `gruvbox-light`,
`nord`, `dracula`, `tokyo-night`, `solarized-dark`, `solarized-light`,
`one-dark`, `rose-pine` e `adwaita-dark`. O padrão do rasi não é um tema
nomeado (`theme = ""`).

```bash
i3-launcher --themes
i3-launcher --theme catppuccin-latte
```

### Teclas

| Tecla | Ação |
| --- | --- |
| `Enter` | executa o item selecionado |
| `Esc` | fecha (código 1) |
| `↑` `↓` / `Ctrl+P` `Ctrl+N` | move a seleção |
| `PgUp` `PgDn` | salta 10 itens |
| `←` `→` | move o item ativo |
| `Ctrl+←` `Ctrl+→` | move o cursor de texto por palavra |
| `Ctrl+B` `Ctrl+F` | move o cursor de texto por caractere |
| `Alt+B` `Alt+F` | move o cursor de texto por palavra |
| `Home` `End` / `Ctrl+A` `Ctrl+E` | início/fim do texto (repetir: primeiro/último item) |
| `Backspace` `Delete` / `Ctrl+H` `Ctrl+D` | apaga caractere |
| `Ctrl+W` `Ctrl+U` `Ctrl+K` | apaga palavra / até o início / até o fim |
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
  double-buffer; grab exclusivo do teclado (o mouse continua livre) e foco
  devolvido à janela que estava ativa ao fechar.
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
- Cantos arredondados/transparência da strip inteira exigem visual ARGB
  (depth 32) + compositor; no X11 sem ARGB ficam quadrados. Os elementos
  internos (seleção, caixa de entrada, ícone-placeholder) têm raio configurável.
- Ícones SVG com `<text>` não renderizam texto (resvg sem a feature `text`).

## Licença

MIT.
