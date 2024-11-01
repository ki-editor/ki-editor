"use strict";(self.webpackChunkdocu=self.webpackChunkdocu||[]).push([[5611],{8622:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>c,contentTitle:()=>s,default:()=>f,frontMatter:()=>r,metadata:()=>a,toc:()=>l});var o=t(4848),i=t(8453);const r={},s="Configurations",a={id:"configurations",title:"Configurations",description:"At the moment, configuration files are not supported, because I'm in favor of compile-time configuration , for the following reasons:",source:"@site/docs/configurations.md",sourceDirName:".",slug:"/configurations",permalink:"/ki-editor/docs/configurations",draft:!1,unlisted:!1,editUrl:"https://github.com/ki-editor/ki-editor/tree/master/docs/configurations.md",tags:[],version:"current",frontMatter:{},sidebar:"tutorialSidebar",previous:{title:"Prompt",permalink:"/ki-editor/docs/components/prompt"},next:{title:"Installation",permalink:"/ki-editor/docs/installation"}},c={},l=[{value:"Files for configurations",id:"files-for-configurations",level:2}];function d(e){const n={a:"a",code:"code",h1:"h1",h2:"h2",header:"header",li:"li",ol:"ol",p:"p",section:"section",sup:"sup",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...(0,i.R)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(n.header,{children:(0,o.jsx)(n.h1,{id:"configurations",children:"Configurations"})}),"\n",(0,o.jsxs)(n.p,{children:["At the moment, configuration files are not supported, because I'm in favor of compile-time configuration ",(0,o.jsx)(n.sup,{children:(0,o.jsx)(n.a,{href:"#user-content-fn-1",id:"user-content-fnref-1","data-footnote-ref":!0,"aria-describedby":"footnote-label",children:"1"})}),", for the following reasons:"]}),"\n",(0,o.jsxs)(n.ol,{children:["\n",(0,o.jsx)(n.li,{children:"Easier to update"}),"\n",(0,o.jsxs)(n.li,{children:["Running with incompatible configurations is impossible ",(0,o.jsx)(n.sup,{children:(0,o.jsx)(n.a,{href:"#user-content-fn-2",id:"user-content-fnref-2","data-footnote-ref":!0,"aria-describedby":"footnote-label",children:"2"})})]}),"\n",(0,o.jsxs)(n.li,{children:["Configuration as code","\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsx)(n.li,{children:"Free type-checking"}),"\n",(0,o.jsxs)(n.li,{children:["Free formatting",(0,o.jsx)(n.sup,{children:(0,o.jsx)(n.a,{href:"#user-content-fn-3",id:"user-content-fnref-3","data-footnote-ref":!0,"aria-describedby":"footnote-label",children:"3"})})]}),"\n",(0,o.jsx)(n.li,{children:"Ability to reduce duplications using functions"}),"\n",(0,o.jsx)(n.li,{children:"Easy backup (fork Ki-editor and push your modified config)"}),"\n"]}),"\n"]}),"\n"]}),"\n",(0,o.jsx)(n.p,{children:"However, I'm open to suggestions, I might even create a new language for that."}),"\n",(0,o.jsx)(n.h2,{id:"files-for-configurations",children:"Files for configurations"}),"\n",(0,o.jsxs)(n.table,{children:[(0,o.jsx)(n.thead,{children:(0,o.jsxs)(n.tr,{children:[(0,o.jsx)(n.th,{children:"Type"}),(0,o.jsx)(n.th,{children:"Path"})]})}),(0,o.jsxs)(n.tbody,{children:[(0,o.jsxs)(n.tr,{children:[(0,o.jsx)(n.td,{children:"Languages"}),(0,o.jsx)(n.td,{children:(0,o.jsx)(n.code,{children:"shared/src/languages.rs"})})]}),(0,o.jsxs)(n.tr,{children:[(0,o.jsx)(n.td,{children:"Theme"}),(0,o.jsx)(n.td,{children:"(Not yet as there is only one theme)"})]})]})]}),"\n","\n",(0,o.jsxs)(n.section,{"data-footnotes":!0,className:"footnotes",children:[(0,o.jsx)(n.h2,{className:"sr-only",id:"footnote-label",children:"Footnotes"}),"\n",(0,o.jsxs)(n.ol,{children:["\n",(0,o.jsxs)(n.li,{id:"user-content-fn-1",children:["\n",(0,o.jsxs)(n.p,{children:["For example, see ",(0,o.jsx)(n.a,{href:"https://wiki.archlinux.org/title/dwm#Configuration",children:"dwm"})," and ",(0,o.jsx)(n.a,{href:"https://xmonad.org/TUTORIAL.html",children:"Xmonad"})," ",(0,o.jsx)(n.a,{href:"#user-content-fnref-1","data-footnote-backref":"","aria-label":"Back to reference 1",className:"data-footnote-backref",children:"\u21a9"})]}),"\n"]}),"\n",(0,o.jsxs)(n.li,{id:"user-content-fn-2",children:["\n",(0,o.jsxs)(n.p,{children:["Neovim usually let's you glide through until it commits kamikaze ",(0,o.jsx)(n.a,{href:"#user-content-fnref-2","data-footnote-backref":"","aria-label":"Back to reference 2",className:"data-footnote-backref",children:"\u21a9"})]}),"\n"]}),"\n",(0,o.jsxs)(n.li,{id:"user-content-fn-3",children:["\n",(0,o.jsxs)(n.p,{children:["Rant: ",(0,o.jsx)(n.a,{href:"https://github.com/toml-lang/toml/issues/532#issuecomment-384313745",children:"TOML does not endorse an official formatter"})," ",(0,o.jsx)(n.a,{href:"#user-content-fnref-3","data-footnote-backref":"","aria-label":"Back to reference 3",className:"data-footnote-backref",children:"\u21a9"})]}),"\n"]}),"\n"]}),"\n"]})]})}function f(e={}){const{wrapper:n}={...(0,i.R)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(d,{...e})}):d(e)}},8453:(e,n,t)=>{t.d(n,{R:()=>s,x:()=>a});var o=t(6540);const i={},r=o.createContext(i);function s(e){const n=o.useContext(r);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function a(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:s(e.components),o.createElement(r.Provider,{value:n},e.children)}}}]);