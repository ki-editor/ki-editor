/*

        ██   ██   ██
        ██   ██   ██         _  ___   _____    _ _ _
        ██   ██   ██        | |/ (_) | ____|__| (_) |_ ___  _ __
        ▀██▄████▄██▀        | ' /| | |  _| / _` | | __/ _ \| '__|
             ██             | . \| | | |__| (_| | | || (_) | |
             ██             |_|\_\_| |_____\__,_|_|\__\___/|_|
        ▄██▀████▀██▄
        ██   ██   ██        Multi-cursor combinatoric modal editor
        ██   ██   ██
        ██   ██   ██

*/

const INTRO: &'static str = "

Welcome to this Ki Tutor aimed at absolute beginners that will teach you
the basics of using the editor.

click i to go to the next line.

Ki is a modal editor meaning it has multiple modes. The editor starts in
normal mode by default (indicated by 'NORM' in the status bar) where you
can change selection and execute actions. To insert Text you will need to
switch to insert mode (indicated by 'INST' in the status bar) by clicking
h (← Insert) or o (Insert →). h (← Insert) will put the cursor at the
beginning of the selection and o (Insert →) will put it at the end of the
selection. Now you can type normally, click the escape key (Esc) to switch
back to normal mode and click Enter to save."

const SPACE_MENU: &'static str = "
                              ╭────────────╮
                              │ Space Menu │
                              ╰────────────╯
Click space to open the main editor menu. Each square in the help menu
represents the position of a key, the bottom text in each square represent
what the key does when you click it, the text in the middle represents
what the key does when you hold shift and click it. The right side
contains shortcuts and actions, for example pick Editor (n) and then Quit
(v).

Space menu reference: https://ki-editor.org/docs/normal-mode/space-menu

You can also click space + / to open the help menu that shows you the main
editor keymap. Core movements are on the right, some quick actions and
selection modes are on the left."

const SELECTION_AND_MOVEMENTS: &'static str = "
                        ╭─────────────────────────╮
                        │ Selection And Movements │
                        ╰─────────────────────────╯
You are currently in the line selection mode (indicated by 'LINE' in the
status bar). Click n (<<) to select previous line and i (>>) for the next
line. Click r to switch to word selection mode. Click n (<<) to select
previous word and i (>>) for the next word. Notice how the same keys (n i)
do different actions depending on the selection mode, these are called
movements, Selection modes share the same movements.

These are the primary selection modes:
 W: Char
 w: Subword
 r: Word
 R: Word* (Big Word)
 a: Line
 A: Line* (Full Line)
 s: Syntax Node
 S: Syntax Node*

and these are the core movements:
 ╭─────┬────────┬─────┬───────────┬─────╮
 │  j  │    l   │  u  │     y     │  ;  │
 │ |<  │    <   │  ^  │     >     │  >| │
 ╰─────┼────────┼─────┼───────────┼─────╯
       │    n   │  e  │     i     │
       │   <<   │  V  │     >>    │
       ├────────┼─────┼───────────┤
       │M: index│     │     .     │
       │m: jump │     │parent Line│
       ╰────────╯     ╰───────────╯

These movements are shared between selection modes, and each one follows a
pattern that will help you discover its function:
╭────────────────┬────────────────┬─────────────────────────╮
│   Movements    │      Name      │         Speed           │
├────────────────┼────────────────┼─────────────────────────┤
│ l (<), y (>)   │ Previous, Next │ Slowest, granular       │
├────────────────┼────────────────┼─────────────────────────┤
│ n (<<), i (>>) │ Left, Right    │ Moderate, commonly used │
├────────────────┼────────────────┼─────────────────────────┤
│ u (^), e (v)   │ Up, Down       │ Fast                    │
├────────────────┼────────────────┼─────────────────────────┤
│ j (|<), ; (>|) │ First, Last    │ Fastest                 │
╰────────────────┴────────────────┴─────────────────────────╯

Index (M) movement allows to go to a specific selection according to its
order. You can for example switch to line selection mode (a), click M
(shift + m), type a line number and then click Enter to go to the line
with that number.

Jump (m) movement allows to go to specific selection just by typing the
letter that appears on the beginning of the desired selection, for
example: switch to word selection mode (r) and then click m. The first
letter of each word will be highlighted, Type the first letter of the word
you want to select. If there are multiple words on screen that starts with
that letter, Each one will have a different letter displayed instead of
the first one, Click the character that is shown at the beginning of the
word to select it.

Parent line (.) movement always moves to the beginning of the last
unindented/detented line before the current one. Here is an example:"

fn main() { // parent line
    println!("first child");
    println!("second child"); // select this line and then click .
}

const REFERENCES: &'static str = "

more about selection modes:
https://ki-editor.org/docs/category/selection-modes-1
and more about movements:
https://ki-editor.org/docs/normal-mode/core-movements
https://ki-editor.org/docs/normal-mode/other-movements

here is a summary of primary selections modes:"

const LINE: &'static str = "
                                 ╭──────╮
                                 │ Line │
                                 ╰──────╯
The only difference between Line and Line* selection modes is that Line*
(Full Line) includes whitespaces at the edges while Line doesn't.

You can switch to line selection mode by clicking a in normal mode or
click A to switch to line* selection mode.

╭────────────────┬──────────────────────────────────────────────╮
│   Movements    │                   Action                     │
├────────────────┼──────────────────────────────────────────────┤
│ l (<), y (>)   │ Previous or next line                        │
├────────────────┼──────────────────────────────────────────────┤
│ n (<<), i (>>) │ Previous or next line (non empty lines only) │
├────────────────┼──────────────────────────────────────────────┤
│ u (^), e (v)   │ Nearest empty line above or below            │
├────────────────┼──────────────────────────────────────────────┤
│ j (|<), ; (>|) │ First or last line                           │
╰────────────────┴──────────────────────────────────────────────╯"

const WORD: &'static str = "
                                 ╭──────╮
                                 │ Word │
                                 ╰──────╯
A word is a sequence of alphanumeric characters including - and _
separated by other symbols or whitespace.

You can switch to word selection mode by clicking r in normal mode.

╭────────────────┬─────────────────────────────────────────────────────╮
│   Movements    │                       Action                        │
├────────────────┼─────────────────────────────────────────────────────┤
│ l (<), y (>)   │ Previous or next word or symbol                     │
├────────────────┼─────────────────────────────────────────────────────┤
│ n (<<), i (>>) │ Previous or next word                               │
├────────────────┼─────────────────────────────────────────────────────┤
│ u (^), e (v)   │ Nearest word or symbol in the previous or next line │
├────────────────┼─────────────────────────────────────────────────────┤
│ j (|<), ; (>|) │ First or last word                                  │
╰────────────────┴─────────────────────────────────────────────────────╯"

const BIG_WORD: &'static str = "
                               ╭──────────╮
                               │ Big Word │
                               ╰──────────╯
A big word is either a sequence of non-whitespace characters or non-
newline whitespace characters or a newline.

You can switch to big word (word*) selection mode by clicking R (shift +
r) in normal mode.

an example of a big word is a
url: https://ki-editor.org/docs/normal-mode/selection-modes/primary#word-1

try to select the white space in the following table using the big word
selection mode:
╭────────────────┬───────────────────────────────────────────────╮
│   Movements    │                       Action                  │
├────────────────┼───────────────────────────────────────────────┤
│ l (<), y (>)   │ Previous or next big word                     │
├────────────────┼───────────────────────────────────────────────┤
│ n (<<), i (>>) │ Previous or next non-whitespace big word      │
├────────────────┼───────────────────────────────────────────────┤
│ u (^), e (v)   │ Nearest big word in the previous or next line │
├────────────────┼───────────────────────────────────────────────┤
│ j (|<), ; (>|) │ First or last non-whitespace big word         │
╰────────────────┴───────────────────────────────────────────────╯"

const SUBWORD: &'static str = "
                                ╭─────────╮
                                │ Subword │
                                ╰─────────╯
A subword is a part of a word as in the following examples:
camelCase PascalCase kebab-case snake_case SCREAMING_CASE

You can switch to subword selection mode by clicking w in normal mode.

╭────────────────┬──────────────────────────────────────────────╮
│   Movements    │                       Action                 │
├────────────────┼──────────────────────────────────────────────┤
│ l (<), y (>)   │ Previous or next subword                     │
├────────────────┼──────────────────────────────────────────────┤
│ n (<<), i (>>) │ Previous or next non-symbol subword          │
├────────────────┼──────────────────────────────────────────────┤
│ u (^), e (v)   │ Nearest subword in the previous or next line │
├────────────────┼──────────────────────────────────────────────┤
│ j (|<), ; (>|) | First or last subword in the current word    │
╰────────────────┴──────────────────────────────────────────────╯"

const CHAR: &'static str = "
                                 ╭──────╮
                                 │ Char │
                                 ╰──────╯
This mode is the most familiar and it's similar to how most editors work.

You can switch to character selection mode by clicking W (shift + w).

╭────────────────┬────────────────────────────────────────────────╮
│   Movements    │                       Action                   │
├────────────────┼────────────────────────────────────────────────┤
│ l (<), y (>)   │ Previous or next character                     │
├────────────────┼────────────────────────────────────────────────┤
│ n (<<), i (>>) │ Previous or next character                     │
├────────────────┼────────────────────────────────────────────────┤
│ u (^), e (v)   │ Nearest character in the previous or next line │
├────────────────┼────────────────────────────────────────────────┤
│ j (|<), ; (>|) | First or last character in the current subword │
╰────────────────┴────────────────────────────────────────────────╯"

const SYNTAX_NODE: &'static str = "
                              ╭─────────────╮
                              │ Syntax Node │
                              ╰─────────────╯
"
