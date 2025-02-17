"use strict";(self.webpackChunkdocu=self.webpackChunkdocu||[]).push([[6659],{750:(e,n,i)=>{i.r(n),i.d(n,{assets:()=>a,contentTitle:()=>d,default:()=>x,frontMatter:()=>r,metadata:()=>c,toc:()=>t});var l=i(4848),o=i(8453),s=i(6787);const r={sidebar_position:3},d="Secondary",c={id:"normal-mode/selection-modes/secondary/index",title:"Secondary",description:"Secondary selection modes are also non-contiguous selection modes.",source:"@site/docs/normal-mode/selection-modes/secondary/index.md",sourceDirName:"normal-mode/selection-modes/secondary",slug:"/normal-mode/selection-modes/secondary/",permalink:"/ki-editor/docs/normal-mode/selection-modes/secondary/",draft:!1,unlisted:!1,editUrl:"https://github.com/ki-editor/ki-editor/tree/master/docs/normal-mode/selection-modes/secondary/index.md",tags:[],version:"current",sidebarPosition:3,frontMatter:{sidebar_position:3},sidebar:"tutorialSidebar",previous:{title:"Primary",permalink:"/ki-editor/docs/normal-mode/selection-modes/primary"},next:{title:"Search Config",permalink:"/ki-editor/docs/normal-mode/search-config"}},a={},t=[{value:"Keymap",id:"keymap",level:2},{value:"Initialization",id:"initialization",level:3},{value:"Local (Forward)",id:"local-forward",level:3},{value:"Local (Backward)",id:"local-backward",level:3},{value:"Global",id:"global",level:3},{value:"Search-related",id:"search-related",level:2},{value:"<code>One</code>",id:"one",level:3},{value:"<code>Last</code>",id:"last",level:3},{value:"<code>Config</code>",id:"config",level:3},{value:"<code>Int</code>",id:"int",level:3},{value:"LSP Diagnostics",id:"lsp-diagnostics",level:2},{value:"<code>All</code>",id:"all",level:3},{value:"<code>Error</code>",id:"error",level:3},{value:"<code>Warn</code>",id:"warn",level:3},{value:"<code>Hint</code>",id:"hint",level:3},{value:"<code>Info</code>",id:"info",level:3},{value:"LSP Location",id:"lsp-location",level:2},{value:"<code>Impl</code>",id:"impl",level:3},{value:"<code>Decl</code>",id:"decl",level:3},{value:"<code>Def</code>",id:"def",level:3},{value:"<code>Type</code>",id:"type",level:3},{value:"<code>Ref-</code>/<code>Ref+</code>",id:"ref-ref",level:3},{value:"Misc",id:"misc",level:2},{value:"<code>Repeat</code>",id:"repeat",level:3},{value:"Example",id:"example",level:4},{value:"<code>Quickfix</code>",id:"quickfix",level:3},{value:"When is global quickfix useful?",id:"when-is-global-quickfix-useful",level:4},{value:"When is local quickfix useful?",id:"when-is-local-quickfix-useful",level:4},{value:"<code>Hunk@</code>/<code>Hunk^</code>",id:"hunkhunk",level:3},{value:"<code>Marks</code>",id:"marks",level:3}];function h(e){const n={br:"br",code:"code",h1:"h1",h2:"h2",h3:"h3",h4:"h4",header:"header",li:"li",ol:"ol",p:"p",pre:"pre",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...(0,o.R)(),...e.components};return(0,l.jsxs)(l.Fragment,{children:[(0,l.jsx)(n.header,{children:(0,l.jsx)(n.h1,{id:"secondary",children:"Secondary"})}),"\n",(0,l.jsx)(n.p,{children:"Secondary selection modes are also non-contiguous selection modes."}),"\n",(0,l.jsx)(n.p,{children:"Secondary selection modes can operate in two scopes:"}),"\n",(0,l.jsxs)(n.ul,{children:["\n",(0,l.jsx)(n.li,{children:"Local: Selections apply only within the current file/buffer you're editing"}),"\n",(0,l.jsx)(n.li,{children:"Global: Selections apply across all files in your workspace/project"}),"\n"]}),"\n",(0,l.jsx)(n.p,{children:"For example, when searching for text:"}),"\n",(0,l.jsxs)(n.ul,{children:["\n",(0,l.jsx)(n.li,{children:"Local search finds matches only in your current file"}),"\n",(0,l.jsx)(n.li,{children:'Global search finds matches in all project files"'}),"\n"]}),"\n",(0,l.jsx)(n.h2,{id:"keymap",children:"Keymap"}),"\n",(0,l.jsx)(n.h3,{id:"initialization",children:"Initialization"}),"\n",(0,l.jsx)(n.p,{children:"Most secondary selection modes are nested below the 3 keybindings below,\nwith the exception of Search and Seacrh Current, which are placed on the\nfirst layer due to their ubiquity."}),"\n",(0,l.jsx)(s.W,{filename:"Secondary Selection Modes Init"}),"\n",(0,l.jsx)(n.p,{children:"Local Find is directional, meaning that if the cursor position does not overlap\nwith any selections of the chosen secondary selection mode, the cursor will\njump to the nearest selection in the chosen direction"}),"\n",(0,l.jsx)(n.p,{children:"Global Find however is non-directional."}),"\n",(0,l.jsx)(n.p,{children:"Notice that the keybindings here are all located on the right side of the keyboard,\nthis is because all the secondary selection modes are placed on the left side of the\nkeyboard, which allows for efficient execution via hand-alternation."}),"\n",(0,l.jsx)(n.p,{children:"There are 3 sets of keymap for secondary selection modes:"}),"\n",(0,l.jsxs)(n.ol,{children:["\n",(0,l.jsx)(n.li,{children:"Local (Forward)"}),"\n",(0,l.jsx)(n.li,{children:"Local (Backward)"}),"\n",(0,l.jsx)(n.li,{children:"Global"}),"\n"]}),"\n",(0,l.jsx)(n.p,{children:"They are almost identical except:"}),"\n",(0,l.jsxs)(n.ol,{children:["\n",(0,l.jsxs)(n.li,{children:[(0,l.jsx)(n.code,{children:"One"})," and ",(0,l.jsx)(n.code,{children:"Int"})," are only applicable for the Local keymaps"]}),"\n",(0,l.jsxs)(n.li,{children:[(0,l.jsx)(n.code,{children:"Search"})," and ",(0,l.jsx)(n.code,{children:"This"})," are only applicable for the Global keymap"]}),"\n",(0,l.jsxs)(n.li,{children:["Position of ",(0,l.jsx)(n.code,{children:"Repeat"})," is different all 3 keymaps to enable easy combo:",(0,l.jsx)(n.br,{}),"\n","a. To repeat the last secondary selection backward, press ",(0,l.jsx)(n.code,{children:"y"})," (Qwerty) twice",(0,l.jsx)(n.br,{}),"\n","b. To repeat the last secondary selection forward, press ",(0,l.jsx)(n.code,{children:"p"})," (Qwerty) twice",(0,l.jsx)(n.br,{}),"\n","c. To repeat the last secondary selection globally, press ",(0,l.jsx)(n.code,{children:"n"})," (Qwerty) twice"]}),"\n"]}),"\n",(0,l.jsx)(n.h3,{id:"local-forward",children:"Local (Forward)"}),"\n",(0,l.jsx)(s.W,{filename:"Secondary Selection Modes (Local Forward)"}),"\n",(0,l.jsx)(n.h3,{id:"local-backward",children:"Local (Backward)"}),"\n",(0,l.jsx)(s.W,{filename:"Secondary Selection Modes (Local Backward)"}),"\n",(0,l.jsx)(n.h3,{id:"global",children:"Global"}),"\n",(0,l.jsx)(s.W,{filename:"Secondary Selection Modes (Global)"}),"\n",(0,l.jsx)(n.h2,{id:"search-related",children:"Search-related"}),"\n",(0,l.jsx)(n.h3,{id:"one",children:(0,l.jsx)(n.code,{children:"One"})}),"\n",(0,l.jsxs)(n.p,{children:["Find one character, this is simlar to Vim's ",(0,l.jsx)(n.code,{children:"f"}),"/",(0,l.jsx)(n.code,{children:"t"}),"."]}),"\n",(0,l.jsx)(n.h3,{id:"last",children:(0,l.jsx)(n.code,{children:"Last"})}),"\n",(0,l.jsx)(n.p,{children:"Repeat the last search."}),"\n",(0,l.jsx)(n.h3,{id:"config",children:(0,l.jsx)(n.code,{children:"Config"})}),"\n",(0,l.jsx)(n.p,{children:"Configure search settings."}),"\n",(0,l.jsx)(n.h3,{id:"int",children:(0,l.jsx)(n.code,{children:"Int"})}),"\n",(0,l.jsx)(n.p,{children:"Integer. Useful for jumping to numbers."}),"\n",(0,l.jsx)(n.h2,{id:"lsp-diagnostics",children:"LSP Diagnostics"}),"\n",(0,l.jsx)(n.h3,{id:"all",children:(0,l.jsx)(n.code,{children:"All"})}),"\n",(0,l.jsx)(n.p,{children:"All diagnostics."}),"\n",(0,l.jsx)(n.h3,{id:"error",children:(0,l.jsx)(n.code,{children:"Error"})}),"\n",(0,l.jsx)(n.p,{children:"Only Diagnostics Error."}),"\n",(0,l.jsx)(n.h3,{id:"warn",children:(0,l.jsx)(n.code,{children:"Warn"})}),"\n",(0,l.jsx)(n.p,{children:"Only Diagnostics Warning."}),"\n",(0,l.jsx)(n.h3,{id:"hint",children:(0,l.jsx)(n.code,{children:"Hint"})}),"\n",(0,l.jsx)(n.p,{children:"Only Diagnostics Hint."}),"\n",(0,l.jsx)(n.h3,{id:"info",children:(0,l.jsx)(n.code,{children:"Info"})}),"\n",(0,l.jsx)(n.p,{children:"Only Diagnostics Information."}),"\n",(0,l.jsx)(n.h2,{id:"lsp-location",children:"LSP Location"}),"\n",(0,l.jsx)(n.h3,{id:"impl",children:(0,l.jsx)(n.code,{children:"Impl"})}),"\n",(0,l.jsx)(n.p,{children:"Implementation."}),"\n",(0,l.jsx)(n.h3,{id:"decl",children:(0,l.jsx)(n.code,{children:"Decl"})}),"\n",(0,l.jsx)(n.p,{children:"Declaration."}),"\n",(0,l.jsx)(n.h3,{id:"def",children:(0,l.jsx)(n.code,{children:"Def"})}),"\n",(0,l.jsx)(n.p,{children:"Definition."}),"\n",(0,l.jsx)(n.h3,{id:"type",children:(0,l.jsx)(n.code,{children:"Type"})}),"\n",(0,l.jsx)(n.p,{children:"Type definition."}),"\n",(0,l.jsxs)(n.h3,{id:"ref-ref",children:[(0,l.jsx)(n.code,{children:"Ref-"}),"/",(0,l.jsx)(n.code,{children:"Ref+"})]}),"\n",(0,l.jsxs)(n.p,{children:[(0,l.jsx)(n.code,{children:"Ref-"}),": References excluding declaration",(0,l.jsx)(n.br,{}),"\n",(0,l.jsx)(n.code,{children:"Ref+"}),": References including declaration"]}),"\n",(0,l.jsxs)(n.p,{children:["In most cases, the Goto selection modes do not make sense in the Local (current\nfile) context, however ",(0,l.jsx)(n.code,{children:"r"})," and ",(0,l.jsx)(n.code,{children:"R"})," are exceptional, because finding local\nreferences are very useful, especially when used in conjunction with Multi-\ncursor."]}),"\n",(0,l.jsx)(n.h2,{id:"misc",children:"Misc"}),"\n",(0,l.jsx)(n.h3,{id:"repeat",children:(0,l.jsx)(n.code,{children:"Repeat"})}),"\n",(0,l.jsx)(n.p,{children:"Repeats the last used secondary selection mode, this is particularly valuable when dealing with scenarios where standard multi-cursor operations are insufficient due to varying modification requirements."}),"\n",(0,l.jsx)(n.h4,{id:"example",children:"Example"}),"\n",(0,l.jsx)(n.p,{children:"When removing unused imports:"}),"\n",(0,l.jsx)(n.pre,{children:(0,l.jsx)(n.code,{className:"language-python",children:"from math import cos  # Unused import 'cos'\nfrom datetime import datetime, date  # Unused import 'date'\n"})}),"\n",(0,l.jsx)(n.p,{children:"In this case, we need t"}),"\n",(0,l.jsxs)(n.ul,{children:["\n",(0,l.jsx)(n.li,{children:"Delete entire first line"}),"\n",(0,l.jsx)(n.li,{children:"Remove only 'date' from second line"}),"\n"]}),"\n",(0,l.jsxs)(n.p,{children:["The ",(0,l.jsx)(n.code,{children:"Repeat"})," command lets you reuse the last selection mode without manual reactivation, making these varied modifications more efficient."]}),"\n",(0,l.jsx)(n.h3,{id:"quickfix",children:(0,l.jsx)(n.code,{children:"Quickfix"})}),"\n",(0,l.jsx)(n.p,{children:"When getting selections using the Global mode, the matches will be stored into\nthe Quickfix List."}),"\n",(0,l.jsx)(n.p,{children:"The quickfix selection mode behaves slightly differently in the Global/Local context:"}),"\n",(0,l.jsxs)(n.table,{children:[(0,l.jsx)(n.thead,{children:(0,l.jsxs)(n.tr,{children:[(0,l.jsx)(n.th,{children:"Context"}),(0,l.jsx)(n.th,{children:"Meaning"})]})}),(0,l.jsxs)(n.tbody,{children:[(0,l.jsxs)(n.tr,{children:[(0,l.jsx)(n.td,{children:"Global"}),(0,l.jsx)(n.td,{children:"Navigate using the current quickfix list"})]}),(0,l.jsxs)(n.tr,{children:[(0,l.jsx)(n.td,{children:"Local"}),(0,l.jsx)(n.td,{children:"Use matches of the current quickfix list that is of the current file"})]})]})]}),"\n",(0,l.jsx)(n.h4,{id:"when-is-global-quickfix-useful",children:"When is global quickfix useful?"}),"\n",(0,l.jsx)(n.p,{children:"When you entered another selection mode but wish to use back the quickfix list."}),"\n",(0,l.jsx)(n.h4,{id:"when-is-local-quickfix-useful",children:"When is local quickfix useful?"}),"\n",(0,l.jsx)(n.p,{children:"When you wanted to use Multi-cursor with the quickfix matches of the current file."}),"\n",(0,l.jsxs)(n.h3,{id:"hunkhunk",children:[(0,l.jsx)(n.code,{children:"Hunk@"}),"/",(0,l.jsx)(n.code,{children:"Hunk^"})]}),"\n",(0,l.jsxs)(n.p,{children:[(0,l.jsx)(n.code,{children:"@"})," means compare against current branch.",(0,l.jsx)(n.br,{}),"\n",(0,l.jsx)(n.code,{children:"^"})," means compare against main/master branch."]}),"\n",(0,l.jsx)(n.p,{children:"Git hunks are the diffs of the current Git repository."}),"\n",(0,l.jsx)(n.p,{children:"It is computed by comparing the current file contents with the content on the latest commit of the current/main branch."}),"\n",(0,l.jsx)(n.p,{children:"This is useful when you want to navigate to your recent changes, but forgot where they are."}),"\n",(0,l.jsx)(n.h3,{id:"marks",children:(0,l.jsx)(n.code,{children:"Marks"})}),"\n",(0,l.jsx)(n.p,{children:"Mark is a powerful feature that allows you to jump to files that contain marks (which can be toggled)."}),"\n",(0,l.jsx)(n.p,{children:"It also allows you to swap two sections of the same file."})]})}function x(e={}){const{wrapper:n}={...(0,o.R)(),...e.components};return n?(0,l.jsx)(n,{...e,children:(0,l.jsx)(h,{...e})}):h(e)}},6401:(e,n,i)=>{i.d(n,{k:()=>j});var l=i(6540),o=i(5293),s=i(6025),r=i(4476),d=i(4848);function c(e,n){const[i,o]=(0,l.useState)((()=>{try{const i=localStorage.getItem(e);return i?JSON.parse(i):n}catch(i){return console.error(`Error reading localStorage key "${e}":`,i),n}}));return(0,l.useEffect)((()=>{try{localStorage.setItem(e,JSON.stringify(i))}catch(n){console.error(`Error writing to localStorage key "${e}":`,n)}}),[e,i]),[i,o]}const a=r.Ik({name:r.Yj(),rows:r.YO(r.YO(r.Ik({normal:r.me(r.Yj()),alted:r.me(r.Yj()),shifted:r.me(r.Yj())}))),keyboard_layouts:r.YO(r.Ik({name:r.Yj(),keys:r.YO(r.YO(r.Yj()))}))}),t="keymap-keyboard-layout",h="keymap-show-keys",x="keymap-split",u="keymap-keys-arrangement",j=e=>{const[n,i]=(0,l.useState)(null),[o,r]=(0,l.useState)(null),c=(0,s.Ay)(`/keymaps/${e.filename}.json`);return(0,l.useEffect)((()=>{(async function(e){try{const n=await fetch(e),i=await n.json();return console.log(i),a.parse(i)}catch(o){r(o)}})(c).then((e=>{i(e)}))}),[]),(0,d.jsxs)("div",{style:{display:"grid",gap:64},children:[n&&(0,d.jsx)(m,{keymap:n}),o&&(0,d.jsx)("div",{style:{color:"red"},children:o.message})]})},p=["Row Staggered","Ortholinear"],m=e=>{const[n,i]=c(h,!0),[s,r]=c(x,!0),[a,j]=c(u,"Ortholinear"),[m,f]=c(t,e.keymap.keyboard_layouts[0].name),y=l.useMemo((()=>e.keymap.keyboard_layouts.find((e=>e.name===m))||e.keymap.keyboard_layouts[0]),[m,e.keymap.keyboard_layouts]),{colorMode:g}=(0,o.G)(),k={width:100,height:100,border:"1px solid "+("light"===g?"black":"white"),display:"grid",placeItems:"center",borderRadius:4,gridTemplateRows:`repeat(${n?4:3}, 1fr)`,fontSize:14},v=()=>(0,d.jsxs)("div",{style:{display:"grid",gridAutoFlow:"column",gap:8,justifyContent:"start",alignItems:"center",overflowX:"auto",whiteSpace:"nowrap",paddingBottom:8},children:[(0,d.jsxs)("label",{children:[(0,d.jsx)("input",{type:"checkbox",checked:n,onChange:()=>i(!n)}),(0,d.jsx)("span",{children:"Show keys"})]}),n&&(0,d.jsx)("select",{value:y.name,onChange:e=>{f(e.target.value)},className:"px-2 py-1 border rounded",children:e.keymap.keyboard_layouts.sort(((e,n)=>e.name.localeCompare(n.name))).map((e=>(0,d.jsx)("option",{value:e.name,children:e.name},e.name)))}),(0,d.jsxs)("label",{children:[(0,d.jsx)("input",{type:"checkbox",checked:s,onChange:()=>r(!s)}),(0,d.jsx)("span",{children:"Split"})]}),(0,d.jsx)("select",{value:a,onChange:e=>{j(e.target.value)},children:p.map((e=>(0,d.jsx)("option",{value:e,children:e},e)))})]}),b=()=>(0,d.jsx)("div",{style:{fontFamily:"sans-serif",whiteSpace:"nowrap",display:"grid",gap:4,paddingBottom:16,overflowX:"auto"},children:e.keymap.rows.map(((e,i)=>(0,d.jsx)("div",{style:{display:"grid",gridAutoFlow:"column",gap:4,justifyContent:"start",marginLeft:"Row Staggered"===a?[0,24,56][i]:0},children:e.map(((e,o)=>(0,d.jsxs)(l.Fragment,{children:[s&&5===o&&(0,d.jsx)("div",{style:{width:100/1.618}}),(0,d.jsx)("div",{style:{textAlign:"center"},children:(0,d.jsxs)("div",{style:{...k,gridArea:"1 / 1",overflow:"hidden",backgroundColor:1!==i||3!=o&&6!=o?void 0:"light"===g?"lightyellow":"darkblue"},children:[n&&(0,d.jsx)("div",{style:{backgroundColor:"light"===g?"black":"white",color:"light"===g?"white":"black",width:"100%"},children:y.keys[i][o]}),e.alted?(0,d.jsxs)("div",{children:["\u2325 ",e.alted]}):(0,d.jsx)("div",{}),e.shifted?(0,d.jsxs)("div",{children:["\u21e7 ",e.shifted]}):(0,d.jsx)("div",{}),e.normal?(0,d.jsx)("div",{children:e.normal}):(0,d.jsx)("div",{})]})})]},`${i}-${o}`)))},i)))});return(0,d.jsxs)("div",{style:{display:"grid",gap:8,marginTop:8,marginBottom:16},children:[(0,d.jsx)(v,{}),(0,d.jsx)(b,{})]})}},6787:(e,n,i)=>{i.d(n,{W:()=>s});i(6540);var l=i(8478),o=i(4848);const s=e=>(0,o.jsx)(l.A,{fallback:(0,o.jsx)("div",{children:"Loading..."}),children:()=>{const n=i(6401).k;return(0,o.jsx)(n,{filename:e.filename})}})}}]);