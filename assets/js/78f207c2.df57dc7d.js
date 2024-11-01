"use strict";(self.webpackChunkdocu=self.webpackChunkdocu||[]).push([[5682],{310:(e,s,n)=>{n.r(s),n.d(s,{assets:()=>c,contentTitle:()=>d,default:()=>m,frontMatter:()=>o,metadata:()=>l,toc:()=>a});var i=n(4848),t=n(8453),r=n(7692);const o={sidebar_position:3},d="Regex-based",l={id:"normal-mode/selection-modes/regex-based",title:"Regex-based",description:"Line",source:"@site/docs/normal-mode/selection-modes/regex-based.mdx",sourceDirName:"normal-mode/selection-modes",slug:"/normal-mode/selection-modes/regex-based",permalink:"/ki-editor/docs/normal-mode/selection-modes/regex-based",draft:!1,unlisted:!1,editUrl:"https://github.com/ki-editor/ki-editor/tree/master/docs/normal-mode/selection-modes/regex-based.mdx",tags:[],version:"current",sidebarPosition:3,frontMatter:{sidebar_position:3},sidebar:"tutorialSidebar",previous:{title:"Syntax Node-based",permalink:"/ki-editor/docs/normal-mode/selection-modes/syntax-node-based"},next:{title:"Local/Global",permalink:"/ki-editor/docs/normal-mode/selection-modes/local-global/"}},c={},a=[{value:"Line",id:"line",level:2},{value:"Full Line",id:"full-line",level:2},{value:"Subword",id:"subword",level:2},{value:"Word",id:"word",level:2},{value:"Column",id:"column",level:2}];function h(e){const s={a:"a",code:"code",h1:"h1",h2:"h2",header:"header",li:"li",ol:"ol",p:"p",section:"section",sup:"sup",ul:"ul",...(0,t.R)(),...e.components};return(0,i.jsxs)(i.Fragment,{children:[(0,i.jsx)(s.header,{children:(0,i.jsx)(s.h1,{id:"regex-based",children:"Regex-based"})}),"\n",(0,i.jsx)(s.h2,{id:"line",children:"Line"}),"\n",(0,i.jsxs)(s.p,{children:["Keybinding: ",(0,i.jsx)(s.code,{children:"e"})]}),"\n",(0,i.jsxs)(s.p,{children:["In this selection mode ",(0,i.jsx)(s.code,{children:"h"}),"/",(0,i.jsx)(s.code,{children:"l"})," behaves exactly like ",(0,i.jsx)(s.code,{children:"j"}),"/",(0,i.jsx)(s.code,{children:"k"}),", and the selection\nis trimmed, which means that the leading and trailing spaces of each line are\nnot selected."]}),"\n",(0,i.jsxs)(s.p,{children:["This is usually used in conjunction with ",(0,i.jsx)(s.code,{children:"i"}),"/",(0,i.jsx)(s.code,{children:"a"})," to immediately enter insert mode at the first/last non-whitespace symbol of the current line."]}),"\n",(0,i.jsx)(s.h2,{id:"full-line",children:"Full Line"}),"\n",(0,i.jsxs)(s.p,{children:["Keybinding: ",(0,i.jsx)(s.code,{children:"E"})]}),"\n",(0,i.jsxs)(s.p,{children:["Same as ",(0,i.jsx)(s.a,{href:"#line",children:"Line"}),", however, leading whitespaces are selected, and trailing whitespaces, including newline characters are also selected."]}),"\n",(0,i.jsx)(s.h2,{id:"subword",children:"Subword"}),"\n",(0,i.jsxs)(s.p,{children:["Keybinding: ",(0,i.jsx)(s.code,{children:"W"})]}),"\n",(0,i.jsx)(s.p,{children:"This selects subwords, even if these words are not separated by spaces."}),"\n",(0,i.jsxs)(s.p,{children:["For example, ",(0,i.jsx)(s.code,{children:"myOatPepperBanana"})," consists of 4 short words, namely: ",(0,i.jsx)(s.code,{children:"my"}),", ",(0,i.jsx)(s.code,{children:"Oat"}),", ",(0,i.jsx)(s.code,{children:"Pepper"})," and ",(0,i.jsx)(s.code,{children:"Banana"}),"."]}),"\n",(0,i.jsxs)(s.p,{children:["This is useful for renaming identifiers, especially if we only want to change a single word of the name. ",(0,i.jsx)(s.sup,{children:(0,i.jsx)(s.a,{href:"#user-content-fn-1",id:"user-content-fnref-1","data-footnote-ref":!0,"aria-describedby":"footnote-label",children:"1"})})]}),"\n",(0,i.jsx)(r.P,{filename:"subword"}),"\n",(0,i.jsx)(s.h2,{id:"word",children:"Word"}),"\n",(0,i.jsxs)(s.p,{children:["Keybinding: ",(0,i.jsx)(s.code,{children:"w"})]}),"\n",(0,i.jsxs)(s.p,{children:["Like ",(0,i.jsx)(s.a,{href:"#subword",children:"Subword"}),", but it treats each word as a sequence of alphanumeric characters (including ",(0,i.jsx)(s.code,{children:"-"})," and ",(0,i.jsx)(s.code,{children:"_"}),")."]}),"\n",(0,i.jsx)(r.P,{filename:"word"}),"\n",(0,i.jsx)(s.h2,{id:"column",children:"Column"}),"\n",(0,i.jsx)(s.p,{children:"Keybindings:"}),"\n",(0,i.jsxs)(s.ul,{children:["\n",(0,i.jsxs)(s.li,{children:[(0,i.jsx)(s.code,{children:"z"}),": Collapse selection (start)"]}),"\n",(0,i.jsxs)(s.li,{children:[(0,i.jsx)(s.code,{children:"$"}),": Collapse selection (end)"]}),"\n"]}),"\n",(0,i.jsxs)(s.p,{children:["In this selection mode, the movements behave like the usual editor, where ",(0,i.jsx)(s.a,{href:"/ki-editor/docs/normal-mode/core-movements#leftright",children:"Left/Right"})," means left/right, and so on."]}),"\n",(0,i.jsxs)(s.p,{children:[(0,i.jsx)(s.a,{href:"/ki-editor/docs/normal-mode/core-movements#firstlast",children:"First/Last"})," means the first/last column of the current line."]}),"\n","\n",(0,i.jsxs)(s.section,{"data-footnotes":!0,className:"footnotes",children:[(0,i.jsx)(s.h2,{className:"sr-only",id:"footnote-label",children:"Footnotes"}),"\n",(0,i.jsxs)(s.ol,{children:["\n",(0,i.jsxs)(s.li,{id:"user-content-fn-1",children:["\n",(0,i.jsxs)(s.p,{children:["This is possible because even Prompt is an editor, so the Word mode also works there. See ",(0,i.jsx)(s.a,{href:"/ki-editor/docs/core-concepts#2-every-component-is-a-buffereditor",children:"Core Concepts"})," ",(0,i.jsx)(s.a,{href:"#user-content-fnref-1","data-footnote-backref":"","aria-label":"Back to reference 1",className:"data-footnote-backref",children:"\u21a9"})]}),"\n"]}),"\n"]}),"\n"]})]})}function m(e={}){const{wrapper:s}={...(0,t.R)(),...e.components};return s?(0,i.jsx)(s,{...e,children:(0,i.jsx)(h,{...e})}):h(e)}},338:(e,s,n)=>{n.d(s,{F:()=>c});var i=n(6540),t=n(6025),r=n(5321),o=n(4476),d=n(4848);const l=o.Ik({description:o.Yj(),steps:o.YO(o.Ik({description:o.Yj(),key:o.Yj(),term_output:o.Yj()})),terminal_height:o.ai(),terminal_width:o.ai(),similar_vim_combos:o.YO(o.Yj())}),c=e=>{const[s,n]=(0,i.useState)([]),[r,c]=(0,i.useState)(null),h=(0,t.Ay)(`/recipes/${e.filename}.json`);return(0,i.useEffect)((()=>{(async function(e){try{const s=await fetch(e),n=await s.json();return o.YO(l).parse(n.recipes_output)}catch(r){c(r)}})(h).then((e=>n(e??[])))}),[]),(0,d.jsxs)("div",{style:{display:"grid",gap:64},children:[(0,d.jsx)("link",{rel:"stylesheet",href:"https://unpkg.com/keyboard-css@1.2.4/dist/css/main.min.css"}),s.map(((e,s)=>(0,d.jsx)(a,{recipe:e},s))),r&&(0,d.jsx)("div",{style:{color:"red"},children:r.message})]})},a=e=>{const s=(0,i.useMemo)((()=>({options:{fontSize:20,cols:e.recipe.terminal_width,rows:e.recipe.terminal_height}})),[]),{instance:n,ref:o}=(0,r.M)(s),[l,c]=(0,i.useState)(0);return(0,i.useEffect)((()=>{const s=e.recipe.steps[l];n?.write(s.term_output)}),[o,n,l]),(0,d.jsxs)("div",{style:{display:"grid",gap:16,justifySelf:"start",overflow:"hidden"},children:[(0,d.jsxs)("div",{style:{display:"grid",gridAutoFlow:"column",alignItems:"center"},children:[(0,d.jsxs)("div",{style:{display:"grid"},children:[(0,d.jsx)("h2",{children:e.recipe.description}),(0,d.jsx)("div",{style:{display:"grid",gridAutoFlow:"column",gap:8,justifyContent:"start"},children:e.recipe.similar_vim_combos.map(((e,s)=>(0,d.jsxs)("div",{style:{display:"grid",gridAutoFlow:"column",gap:4,justifyContent:"start",alignItems:"center"},children:[(0,d.jsx)("img",{style:{height:24},src:(0,t.Ay)("/img/vim-icon.svg")}),(0,d.jsx)("code",{style:{padding:"0 8px"},children:e})]},s)))})]}),(0,d.jsxs)("div",{style:{display:"grid",gap:8,gridAutoFlow:"column",justifySelf:"end"},children:[(0,d.jsx)("button",{className:"kbc-button",onClick:()=>c(Math.max(l-1,0)),children:"\u2039"}),(0,d.jsx)("button",{className:"kbc-button",onClick:()=>c(Math.min(l+1,e.recipe.steps.length-1)),children:"\u203a"})]})]}),(0,d.jsx)("div",{ref:o,style:{justifySelf:"start",border:"1px solid black"}}),(0,d.jsx)("div",{style:{display:"grid",justifyContent:"start",alignContent:"start",justifyItems:"center",gap:8},children:(0,d.jsx)("div",{style:{display:"grid",gap:2,gridAutoFlow:"column",justifySelf:"start",overflowX:"auto",width:"100%"},children:e.recipe.steps.map(((e,s)=>(0,d.jsx)("button",{onClick:()=>c(s),className:["kbc-button",s===l?"active":void 0].join(" "),style:{fontFamily:"monospace"},children:e.key})))})})]})}},7692:(e,s,n)=>{n.d(s,{P:()=>r});n(6540);var i=n(8478),t=n(4848);const r=e=>(0,t.jsx)(i.A,{fallback:(0,t.jsx)("div",{children:"Loading..."}),children:()=>{const s=n(338).F;return(0,t.jsx)(s,{filename:e.filename})}})}}]);