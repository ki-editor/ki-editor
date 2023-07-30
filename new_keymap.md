# Selection Mode + hjkl

ctrl+j/k go to next/prev item in dropdown in insert mode

shift+j the go to last
shift+k the go to first

nlsc = next line same column

| Selection Mode       | j                    | k                    | h                   | l                    |
| -------------------- | -------------------- | -------------------- | ------------------- | -------------------- |
| Node                 | next sibling         | prev sibling         | parent              | child                |
| Line                 | next line            | prev line            | ?                   | ?                    |
| Scroll               | scroll down          | scroll up            | scroll left         | scroll right         |
| Window               | move to below window | move to above window | move to left window | move to right window |
| Opened file          | ?                    | ?                    | previous file       | next file            |
| Quickfix list        | go to next item      | go to prev item      | go te prev list     | go to next list      |
| Token                | nlsc                 | nlsc                 | prev                | next                 |
| Character            | nlsc                 | nlsc                 | prev                | next                 |
| Word                 | nlsc                 | nlsc                 | prev                | next                 |
| Diagnostic (error)   | nlsc                 | nlsc                 | prev                | next                 |
| Diagnostic (warning) | nlsc                 | nlsc                 | prev                | next                 |
| Diagnostic (info)    | nlsc                 | nlsc                 | prev                | next                 |
| Match (literal)      | nlsc                 | nlsc                 | prev                | next                 |
| Match (AST grep)     | nlsc                 | nlsc                 | prev                | next                 |
| Match (regex)        | nlsc                 | nlsc                 | prev                | next                 |
| Git hunk (regex)     | nlsc                 | nlsc                 | prev                | next                 |

Action is also a mode, need to press Esc to go back to Navigation mode.

| Action               | j   | k   | h                                        | l                                        |
| -------------------- | --- | --- | ---------------------------------------- | ---------------------------------------- |
| Navigation (default) | ?   | ?   |                                          |
| Insert               | ?   | ?   | Enter insert mode before cursor          | Enter insert mode after cursor           |
| Delete               | ?   | ?   | Delete prev object                       | Delete next object                       |
| Exchange             | ?   | ?   | Exchange current object with prev object | Exchange current object with next object |
| Add cursor           | ?   | ?   | Add prev object                          | Add next object                          |
