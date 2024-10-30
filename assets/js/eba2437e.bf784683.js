"use strict";(self.webpackChunkdocu=self.webpackChunkdocu||[]).push([[112],{2293:(e,n,i)=>{i.r(n),i.d(n,{assets:()=>o,contentTitle:()=>l,default:()=>x,frontMatter:()=>d,metadata:()=>c,toc:()=>h});var s=i(4848),r=i(8453),t=i(7692);const d={},l="Actions",c={id:"normal-mode/actions/index",title:"Actions",description:"Notes for reading",source:"@site/docs/normal-mode/actions/index.mdx",sourceDirName:"normal-mode/actions",slug:"/normal-mode/actions/",permalink:"/ki-editor/docs/normal-mode/actions/",draft:!1,unlisted:!1,editUrl:"https://github.com/ki-editor/ki-editor/tree/master/docs/normal-mode/actions/index.mdx",tags:[],version:"current",frontMatter:{},sidebar:"tutorialSidebar",previous:{title:"Misc",permalink:"/ki-editor/docs/normal-mode/selection-modes/local-global/misc"},next:{title:"Clipboard-related Actions",permalink:"/ki-editor/docs/normal-mode/actions/clipboard-related-actions"}},o={},h=[{value:"Notes for reading",id:"notes-for-reading",level:2},{value:"Enter insert mode",id:"enter-insert-mode",level:2},{value:"Open",id:"open",level:2},{value:"Delete",id:"delete",level:2},{value:"Change",id:"change",level:2},{value:"Replace with previous/next copied text",id:"replace-with-previousnext-copied-text",level:2},{value:"Replace with pattern",id:"replace-with-pattern",level:2},{value:"Raise",id:"raise",level:2},{value:"Join",id:"join",level:2},{value:"Transform",id:"transform",level:2},{value:"Save",id:"save",level:2},{value:"Undo/Redo",id:"undoredo",level:2}];function a(e){const n={a:"a",br:"br",code:"code",h1:"h1",h2:"h2",header:"header",li:"li",ol:"ol",p:"p",pre:"pre",strong:"strong",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...(0,r.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.header,{children:(0,s.jsx)(n.h1,{id:"actions",children:"Actions"})}),"\n",(0,s.jsx)(n.h2,{id:"notes-for-reading",children:"Notes for reading"}),"\n",(0,s.jsxs)(n.ol,{children:["\n",(0,s.jsx)(n.li,{children:'When "selection" is mentioned, you should read it as "selection(s)", because\nthese actions work with multiple cursors.'}),"\n"]}),"\n",(0,s.jsxs)(n.h2,{id:"enter-insert-mode",children:["Enter ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/insert-mode/",children:"insert mode"})]}),"\n",(0,s.jsx)(n.p,{children:"Keybindings:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"i"}),": Enter insert mode before selection"]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"a"}),": Enter insert mode after selection"]}),"\n"]}),"\n",(0,s.jsx)(n.h2,{id:"open",children:"Open"}),"\n",(0,s.jsx)(n.p,{children:"Keybindings:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"o"}),": Open after selection"]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"O"}),": Open before selection"]}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["If the current selection mode is ",(0,s.jsx)(n.strong,{children:"not"})," ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/selection-modes/#contiguity",children:"contiguous"}),",\nthen ",(0,s.jsx)(n.code,{children:"o"}),"/",(0,s.jsx)(n.code,{children:"O"})," inserts one space after/before the current\nselection."]}),"\n",(0,s.jsx)(n.p,{children:"Otherwise, it inserts a gap before/after the current selection, and enter Insert mode."}),"\n",(0,s.jsx)(n.p,{children:"For example, consider the following Javascript code:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:"hello(x, y);\n"})}),"\n",(0,s.jsxs)(n.p,{children:["Assuming the current selection mode is ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/selection-modes/syntax-node-based#syntax-node",children:"Syntax Node"}),", and the current selection is ",(0,s.jsx)(n.code,{children:"y"}),", pressing ",(0,s.jsx)(n.code,{children:"o"})," results in the following (Note that ",(0,s.jsx)(n.code,{children:"\u2502"})," represents the cursor):"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:"hello(x, y, \u2502);\n"})}),"\n",(0,s.jsx)(n.h2,{id:"delete",children:"Delete"}),"\n",(0,s.jsx)(n.p,{children:"Keybindings:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"d"}),": Delete until next selection"]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"D"}),": Delete until previous selection"]}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["This deletes the current selection(s), however, if the current selection mode is\n",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/selection-modes/#contiguity",children:"contiguous"}),", it will delete until the\nnext/previous selection, and selects the next/previous selection."]}),"\n",(0,s.jsx)(n.p,{children:"But, if the current selection is the last/first selection, it will delete until the\nprevious/next selection instead, and selects the previous/next selection."}),"\n",(0,s.jsx)(n.p,{children:"For example, consider the following Javascript code:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:"hello(x, y);\n"})}),"\n",(0,s.jsxs)(n.p,{children:["Assuming the current selection mode is ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/selection-modes/syntax-node-based#syntax-node",children:"Syntax Node"}),", and the current selection is ",(0,s.jsx)(n.code,{children:"x"}),", pressing ",(0,s.jsx)(n.code,{children:"d"})," results in the following:"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-js",children:"hello(y);\n"})}),"\n",(0,s.jsx)(n.h2,{id:"change",children:"Change"}),"\n",(0,s.jsx)(n.p,{children:"Keybindings:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"c"}),": Change"]}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["This deletes the current selected text, and enter ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/insert-mode/",children:"Insert mode\n"}),"."]}),"\n",(0,s.jsx)(n.h2,{id:"replace-with-previousnext-copied-text",children:"Replace with previous/next copied text"}),"\n",(0,s.jsx)(n.p,{children:"Keybindings:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"ctrl+n"}),": Replace current selection with next copied text in the clipboard history"]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"ctrl+p"}),": Replace current selection with previous copied text in the clipboard history"]}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["This is similar to ",(0,s.jsx)(n.a,{href:"https://www.gnu.org/software/emacs/manual/html_node/emacs/Earlier-Kills.html",children:"Yanking Earlier Kills"})," in Emacs."]}),"\n",(0,s.jsx)(n.p,{children:"This is useful when you want to retrieve earlier copies."}),"\n",(0,s.jsx)(n.h2,{id:"replace-with-pattern",children:"Replace with pattern"}),"\n",(0,s.jsxs)(n.p,{children:["Keybinding: ",(0,s.jsx)(n.code,{children:"ctrl+r"})]}),"\n",(0,s.jsxs)(n.p,{children:["This replaces the current selection using the search pattern and replacement\npattern specified in the ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/selection-modes/local-global/text-search#configurator",children:"Text Search Configurator"}),"."]}),"\n",(0,s.jsx)(n.p,{children:"For example:"}),"\n",(0,s.jsxs)(n.table,{children:[(0,s.jsx)(n.thead,{children:(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.th,{children:"Mode"}),(0,s.jsx)(n.th,{children:"Selected text"}),(0,s.jsx)(n.th,{children:"Search"}),(0,s.jsx)(n.th,{children:"Replacement"}),(0,s.jsx)(n.th,{children:"Result"})]})}),(0,s.jsxs)(n.tbody,{children:[(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:"Literal"}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"f"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"f"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"g"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"g(x)"})})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:"Regex"}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:'"yo"'})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:'"(.*)"'})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"[$1]"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"[yo]"})})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:"AST Grep"}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"f(x)"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"f($Z)"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"$Z(f)"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"x(f)"})})]}),(0,s.jsxs)(n.tr,{children:[(0,s.jsx)(n.td,{children:"Case Agnostic"}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"a_bu"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"a bu"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"to li"})}),(0,s.jsx)(n.td,{children:(0,s.jsx)(n.code,{children:"to_li"})})]})]})]}),"\n",(0,s.jsx)(n.h2,{id:"raise",children:"Raise"}),"\n",(0,s.jsxs)(n.p,{children:["Keybinding: ",(0,s.jsx)(n.code,{children:"T"})]}),"\n",(0,s.jsxs)(n.p,{children:["This is one of my favorite actions, it only works for ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/selection-modes/syntax-node-based#syntax-node",children:"syntax node"})," selection modes."]}),"\n",(0,s.jsx)(n.p,{children:"This replaces the parent node of the current node, with the current node."}),"\n",(0,s.jsx)(t.P,{filename:"raise"}),"\n",(0,s.jsx)(n.p,{children:"Note: Raise should never cause any syntax errors, if it does that's a bug."}),"\n",(0,s.jsx)(n.h2,{id:"join",children:"Join"}),"\n",(0,s.jsxs)(n.p,{children:["Keybinding: ",(0,s.jsx)(n.code,{children:"J"})]}),"\n",(0,s.jsx)(n.p,{children:"Joins multiple lines within the current selection(s) into a single line."}),"\n",(0,s.jsx)(t.P,{filename:"join"}),"\n",(0,s.jsx)(n.h2,{id:"transform",children:"Transform"}),"\n",(0,s.jsxs)(n.p,{children:["Keybinding: ",(0,s.jsx)(n.code,{children:"!"})]}),"\n",(0,s.jsx)(n.p,{children:"Transformative actions are nested under here, such as (non-exhaustive):"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"w"}),": Wrap (Wrap current selection into multiple lines)"]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"l"}),": Convert to ",(0,s.jsx)(n.code,{children:"lower case"})]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"s"}),": Convert to ",(0,s.jsx)(n.code,{children:"snake_case"})]}),"\n"]}),"\n",(0,s.jsx)(n.h2,{id:"save",children:"Save"}),"\n",(0,s.jsxs)(n.p,{children:["Keybinding: ",(0,s.jsx)(n.code,{children:"enter"}),(0,s.jsx)(n.br,{}),"\n","Reason: The ",(0,s.jsx)(n.code,{children:"esc enter"})," combo is sweet."]}),"\n",(0,s.jsx)(n.p,{children:"Upon saving, formatting will be applied if possible."}),"\n",(0,s.jsxs)(n.p,{children:["After formatting, the ",(0,s.jsx)(n.a,{href:"/ki-editor/docs/normal-mode/core-movements#current",children:"Current"})," movement will be executed, to reduce disorientation caused by the misplaced selection due to content changes."]}),"\n",(0,s.jsx)(n.h2,{id:"undoredo",children:"Undo/Redo"}),"\n",(0,s.jsx)(n.p,{children:"Keybindings:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"u"}),": Undo"]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"U"}),": Redo"]}),"\n"]}),"\n",(0,s.jsx)(n.p,{children:"Notes:"}),"\n",(0,s.jsxs)(n.ol,{children:["\n",(0,s.jsx)(n.li,{children:"Undo/redo works for multi-cursors as well"}),"\n",(0,s.jsx)(n.li,{children:"The current implementation is naive, it undoes/redoes character-by-character, instead of chunk-by-chunk, so it can be mildly frustrating"}),"\n"]})]})}function x(e={}){const{wrapper:n}={...(0,r.R)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(a,{...e})}):a(e)}},338:(e,n,i)=>{i.d(n,{F:()=>o});var s=i(6540),r=i(6025),t=i(5321),d=i(4476),l=i(4848);const c=d.Ik({description:d.Yj(),steps:d.YO(d.Ik({description:d.Yj(),key:d.Yj(),term_output:d.Yj()})),terminal_height:d.ai(),terminal_width:d.ai(),similar_vim_combos:d.YO(d.Yj())}),o=e=>{const[n,i]=(0,s.useState)([]),[t,o]=(0,s.useState)(null),a=(0,r.Ay)(`/recipes/${e.filename}.json`);return(0,s.useEffect)((()=>{(async function(e){try{const n=await fetch(e),i=await n.json();return d.YO(c).parse(i.recipes_output)}catch(t){o(t)}})(a).then((e=>i(e??[])))}),[]),(0,l.jsxs)("div",{style:{display:"grid",gap:64},children:[(0,l.jsx)("link",{rel:"stylesheet",href:"https://unpkg.com/keyboard-css@1.2.4/dist/css/main.min.css"}),n.map(((e,n)=>(0,l.jsx)(h,{recipe:e},n))),t&&(0,l.jsx)("div",{style:{color:"red"},children:t.message})]})},h=e=>{const n=(0,s.useMemo)((()=>({options:{fontSize:20,cols:e.recipe.terminal_width,rows:e.recipe.terminal_height}})),[]),{instance:i,ref:d}=(0,t.M)(n),[c,o]=(0,s.useState)(0);return(0,s.useEffect)((()=>{const n=e.recipe.steps[c];i?.write(n.term_output)}),[d,i,c]),(0,l.jsxs)("div",{style:{display:"grid",gap:16,justifySelf:"start",overflow:"hidden"},children:[(0,l.jsxs)("div",{style:{display:"grid",gridAutoFlow:"column",alignItems:"center"},children:[(0,l.jsxs)("div",{style:{display:"grid"},children:[(0,l.jsx)("h2",{children:e.recipe.description}),(0,l.jsx)("div",{style:{display:"grid",gridAutoFlow:"column",gap:8,justifyContent:"start"},children:e.recipe.similar_vim_combos.map(((e,n)=>(0,l.jsxs)("div",{style:{display:"grid",gridAutoFlow:"column",gap:4,justifyContent:"start",alignItems:"center"},children:[(0,l.jsx)("img",{style:{height:24},src:(0,r.Ay)("/img/vim-icon.svg")}),(0,l.jsx)("code",{style:{padding:"0 8px"},children:e})]},n)))})]}),(0,l.jsxs)("div",{style:{display:"grid",gap:8,gridAutoFlow:"column",justifySelf:"end"},children:[(0,l.jsx)("button",{className:"kbc-button",onClick:()=>o(Math.max(c-1,0)),children:"\u2039"}),(0,l.jsx)("button",{className:"kbc-button",onClick:()=>o(Math.min(c+1,e.recipe.steps.length-1)),children:"\u203a"})]})]}),(0,l.jsx)("div",{ref:d,style:{justifySelf:"start",border:"1px solid black"}}),(0,l.jsx)("div",{style:{display:"grid",justifyContent:"start",alignContent:"start",justifyItems:"center",gap:8},children:(0,l.jsx)("div",{style:{display:"grid",gap:2,gridAutoFlow:"column",justifySelf:"start",overflowX:"auto",width:"100%"},children:e.recipe.steps.map(((e,n)=>(0,l.jsx)("button",{onClick:()=>o(n),className:["kbc-button",n===c?"active":void 0].join(" "),style:{fontFamily:"monospace"},children:e.key})))})})]})}},7692:(e,n,i)=>{i.d(n,{P:()=>t});i(6540);var s=i(8478),r=i(4848);const t=e=>(0,r.jsx)(s.A,{fallback:(0,r.jsx)("div",{children:"Loading..."}),children:()=>{const n=i(338).F;return(0,r.jsx)(n,{filename:e.filename})}})}}]);