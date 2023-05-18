- [x] scrolling view
- [x] swapping?
- [x] transactional edit: so that exchanging siblings can be undone in one step
      instead of two smaller steps
- [x] multi-cursor
- [x] swap d to ^x, y to ^c
- [x] ^f for find
- [x] f/F for moving forward/backward (so that it's consistent with other
      actions, which is selection mode first, then action)
- [x] moving selection should scroll the page when out of view
- [x] remove windows, as it is useless
- [x] e for eating forward, useful for replacing current node with parent, also might make `d` obsolete
- [x] implement buffer
- [x] use patch for undo (undo history should be stored under buffer) (https://docs.rs/diffy/0.3.0/diffy/)
- [x] implement component
- [] LSP (after window system implemented)
- [] split window (needed for autocomplete, prompt)
  - parent-child architecture
  - each window can have multiple children window
  - when a window is closed, all of its children are closed
  - each window have a group, if any of the window in the same group is closed
    all windows in the same group should be closed
- [] g for selecting the next node that is the same generation (descendant
  level from root) as the current node
- [] e for elevate the current node such that it becomes the siblings of its parent
- [] e for enclose the current node with one of the brackets
- [] ([{ for enclosing current selection with brackets
- [] f for moving to the next node which has the same field name as the current node
- [] mechanism for adding selection to all matching selection within current selection
- [] incorporate AST grep (https://github.com/ast-grep/ast-grep), the result is not very satisfying
- [] multi eat parent should not proceed if the final edit overlaps (not too important because we use patch for undo/redo now, so messed up stuff can be undone)
- [] jump should work for multiple selection
- [] incorporate first-class refactoring (https://github.com/ThePrimeagen/refactoring.nvim)
- [] file picker
- [] file tree
- [] autocomplete
- [] engine: press Enter to open new line below
