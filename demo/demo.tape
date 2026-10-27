# Records demo/demo.gif. Requires vhs (https://github.com/charmbracelet/vhs) and pks on PATH.
#   brew install vhs
#   vhs demo/demo.tape
Output demo/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 640
Set Padding 20
Set TypingSpeed 70ms
Set Theme "Catppuccin Mocha"

Hide
Type "cd examples/monorepo/apps/web && clear" Enter
Show

Type "pks" Enter
Sleep 2s
Down 4
Sleep 1s
Type "ui"
Sleep 1.5s
Enter
Sleep 2.5s

Type "pks lint" Enter
Sleep 3s
