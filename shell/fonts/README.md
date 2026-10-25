The fonts Vetro's shell plugins need, installed to ~/.local/share/fonts/vetro
by install.sh:

- D-DIN, D-DIN Bold, D-DIN Exp, D-DIN Exp Bold: Datto, SIL Open Font License
  (see OFL-1.1.txt and FONTLOG.txt here).
- Nunito.ttf: Nunito (variable), The Nunito Project Authors, SIL Open Font
  License (see Nunito-OFL.txt). The lock screen's rounded type.
- MaterialSymbolsOutlined.ttf, MaterialSymbolsRounded.ttf: Material Symbols
  (variable), Google, Apache License 2.0. The bar and menus draw their icons
  with Outlined; the launch and weather cards with Rounded.

The launch addon's cards carry their own copy of D-DIN (the QML loads it by a
relative path), and so does the login screen (the sddm user cannot see your
home folder).
