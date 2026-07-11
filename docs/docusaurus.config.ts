import type * as Preset from "@docusaurus/preset-classic";
import type { Config } from "@docusaurus/types";
import { themes as prismThemes } from "prism-react-renderer";

const config: Config = {
    title: "Ki Editor",
    tagline: "Multi-cursor structural editor",
    favicon: "img/favicon.png",

    // Set the production url of your site here
    url: "https://ki-editor.org",
    // Set the /<baseUrl>/ pathname under which your site is served
    // For GitHub pages deployment, it is often '/<projectName>/'
    baseUrl: "/",
    // GitHub pages deployment config.
    // If you aren't using GitHub pages, you don't need these.
    organizationName: "ki-editor", // Usually your GitHub org/user name.
    projectName: "ki-editor", // Usually your repo name.

    onBrokenLinks: "throw",
    onBrokenMarkdownLinks: "throw",
    onBrokenAnchors: "throw",
    onDuplicateRoutes: "throw",
    // Even if you don't use internationalization, you can use this field to set
    // useful metadata like html lang. For example, if your site is Chinese, you
    // may want to replace "en" with "zh-Hans".
    i18n: {
        defaultLocale: "en",
        locales: ["en"],
    },

    presets: [
        [
            "classic",
            {
                docs: {
                    sidebarPath: "./sidebars.ts",
                    // Please change this to your repo.
                    // Remove this to remove the "edit this page" links.
                    editUrl:
                        "https://github.com/ki-editor/ki-editor/tree/master",
                },
                blog: {
                    showReadingTime: true,
                    feedOptions: {
                        type: ["rss", "atom"],
                        xslt: true,
                    },
                    // Please change this to your repo.
                    // Remove this to remove the "edit this page" links.
                    editUrl:
                        "https://github.com/ki-editor/ki-editor/tree/master",
                    // Useful options to enforce blogging best practices
                    onInlineTags: "warn",
                    onInlineAuthors: "warn",
                    onUntruncatedBlogPosts: "warn",
                },
                theme: {
                    customCss: "./src/css/custom.css",
                },
            } satisfies Preset.Options,
        ],
    ],

    themeConfig: {
        // Replace with your project's social card
        image: "img/tree-seal-script.svg",
        navbar: {
            title: "Ki Editor",
            hideOnScroll: true,
            logo: {
                alt: "Ki Logo",
                src: "img/logo.png",
                srcDark: "img/logo-white.png",
            },
            items: [
                {
                    type: "docSidebar",
                    sidebarId: "docSidebar",
                    position: "left",
                    label: "Docs",
                    className: "navbar__link--docs",
                },
                {
                    position: "left",
                    label: "Introduction",
                    to: "/docs/introduction",
                    className: "navbar__link--intro",
                },
                {
                    position: "left",
                    label: "Blog",
                    to: "/blog",
                    className: "navbar__link--blog",
                },
                {
                    position: "right",
                    html: "Chat with <b>Ki</b>mmunity",
                    href: "https://ki-editor.zulipchat.com/join/zzhagqzl6wyzpqfeqxcsrkin/",
                    className: "navbar__link--zulip",
                },
                {
                    position: "right",
                    label: "Source Code",
                    href: "https://github.com/ki-editor/ki-editor",
                    className: "navbar__link--github",
                },
                {
                    position: "right",
                    label: "Download",
                    href: "https://github.com/ki-editor/ki-editor/releases/tag/latest",
                    className: "navbar__link--download",
                },
            ],
        },
        footer: {
            style: "dark",
            links: [
                {
                    title: "Docs",
                    items: [
                        {
                            label: "Docs",
                            to: "/docs/introduction",
                        },
                    ],
                },
                {
                    title: "Community",
                    items: [
                        {
                            html: `<svg style="display:inline;vertical-align:middle;margin-right:6px;" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H6l-2 2V4h16v12z"/></svg> Chat with Kimmunity`,
                        },
                    ],
                },
                {
                    title: "More",
                    items: [
                        {
                            label: "Blog",
                            to: "/blog",
                        },
                        {
                            html: `<svg style="display:inline;vertical-align:middle;margin-right:6px;" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M9.4 16.6L4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4zm5.2 0L19.2 12l-4.6-4.6L16 6l6 6-6 6-1.4-1.4z"/></svg> Source Code`,
                        },
                    ],
                },
            ],
            copyright: `Copyright © ${new Date().getFullYear()} Ki Editor.`,
        },
        prism: {
            theme: prismThemes.github,
            darkTheme: prismThemes.dracula,
        },
    } satisfies Preset.ThemeConfig,

    // Refer https://github.com/praveenn77/docusaurus-lunr-search
    plugins: [require.resolve("docusaurus-lunr-search")],
    themes: ["docusaurus-json-schema-plugin"],
};

export default config;
