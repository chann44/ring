# Run from the repository root: cargo build --release --locked && vhs assets/demo.tape
Output assets/demo.gif

Require bash

Set Shell "bash"
Set FontSize 18
Set Width 1000
Set Height 800
Set Padding 20
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 12
Set Framerate 15
Set TypingSpeed 100ms

Hide
Type `export PATH="$PWD/target/release:$PATH"; export PS1='$ '; clear`
Enter
Sleep 500ms
Show

Type "ring"
Sleep 500ms
Enter
Sleep 25s
Type "q"
Sleep 1s
