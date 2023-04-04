## Consistency

lowercase alphabet for moving forward (place cursor at last character of object),
uppercase alphabet for moving backward (place cursor at first character of object)

## Object

m = (m)atching (used with find)

0 = nothing (collapse selection)
a = (a)lphabet (or grapheme)
w = syntactically significant (w)ord
l = (l)ine

p = (p)arent node
k = (k)id node
n = sibling (n)ode

s = (s)plit
t = (t)ab

<key of the action> = nothing (for example, `rr` means replace nothing with yanked selection)

f<text><Enter> = Find text forward
F<text><Enter> = Find text backward

Cursor is also an object.

---

## Action

### Xchange

x = exchange current object with next object

For example, `lx` means to swap the line downward with another line
For example, `Px` means to move the current node upward.

### Delete

d = Delete the current object
D = Delete all other object except the current object.

### Replace

r = Replace the current selection with yanked selection

### Add Cursor

c = add cursor to the next object
C = add cursor to the previous object

For example:

- `mc` means add cursor to the next matching text
- `lc` means add cursor to the next line
- `nc` means add cursor to the next sibling node

### Open

o<object> = Open a new object

For example, `os` means open a new split. `on` means open o new node.

### Insert

i = delete object and enter insert mode (no distinction of before or after cursor, because in every mode, the cursor is caret)

### Yank

y = yank the current object

## Normal mode command (non movement)

### Dot (repeat last action)

### Find

f = find (press Enter to proceed to next search, <Ctrl-c> to exit prompt, <Esc>q)
F = reverse find = match every selection that does not match the input regex

### Jump

j<object> = jump to object
Like quick motion, but you can choose where you want to go.
For example, pressing `jl`, all the lines after cursor will have a label at the
last character of each line.
Pressing the character move to that line.

### End

e = go to the last object

### Undo

u = undo
U = redo

### Go to mode

gd = Go to definition
gr = Go to references
gf = Go to file

### View mode

zt
zb
zz

---

## Highlight mode

Highlight mode starts with a caret-cursor instead of a box-cursor, in other words,
pressing h does not select anything at first.

## Highlight mode command

h<movement>

y = (y)ank
d = (d)elete
r = (r)eplace
c<object> = (c)ursor = add cursor to all object in the current selection

## Question?

- How to join line?
- How to open new line? `lo`
- How to surrond?

## Glossary

a = object - alphabet
b = backward
c = action - add cursor
d = action - delete
e = movement - end
f = action - find
g = mode g
h = highlight mode
i = action - insert
j = action - jump
k = object - kid node
l = object - line
m = object - matching text
n =
o = action - open
p = object - parent node
q =
r = action - replace
s = object - sibling node
t = non-text object - tab
u = action - undo
v =
w = object - word
x = action - exchange
y = action - yank
z =

yank,delete,parent node,sibling node,child node,word,swap,line,find,cursor (as in computer graphics),grapheme,jump,highlight,open,replace,insert,undo,next,previous,matching text

| Alphabet | Word                      | Explanation |
| -------- | ------------------------- | ----------- |
| y        | yank                      |             |
| d        | delete                    |             |
| p        | parent node               |             |
| s        | sibling node              |             |
| c        | child node                |             |
| w        | word                      |             |
| t        | swap (or trade)           | trade       |
| l        | line                      |             |
| f        | find                      |             |
| g        | cursor (or glider)        | glider      |
| r        | grapheme (or rune)        | rune        |
| j        | jump                      |             |
| h        | highlight (or illuminate) | illuminate  |
| o        | open                      |             |
| e        | replace (or exchange)     | exchange    |
| i        | insert (or nestle)        | nestle      |
| u        | undo                      |             |
| n        | next (or subsequent)      | subsequent  |
| v        | previous (or former)      | former      |
| m        | matching text (or match)  | match       |
