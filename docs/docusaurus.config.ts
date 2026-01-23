import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

const config: Config = {
    title: "Ki Editor",
    tagline: "Multi-cursor structural editor",
    favicon: "img/favicon.ico",

    // Set the production url of your site here
    url: "https://ki-editor.org",
    // Set the /<baseUrl>/ pathname under which your site is served
    // For GitHub pages deployment, it is often '/<projectName>/'
    baseUrl: "/",

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
            logo: {
                alt: "Ki Logo",
                src: "img/logo.svg",
                srcDark: "img/logo-white.svg",
            },
            items: [
                {
                    type: "docSidebar",
                    sidebarId: "docSidebar",
                    position: "left",
                    label: "Docs",
                },
                {
                    position: "left",
                    label: "Introduction",
                    to: "/docs/introduction",
                },
                {
                    position: "left",
                    label: "Blog",
                    to: "/blog",
                },
                {
                    position: "right",
                    label: "Chat (Zulip)",
                    href: "https://ki-editor.zulipchat.com/join/zzhagqzl6wyzpqfeqxcsrkin/",
                },
                {
                    position: "right",
                    label: "GitHub",
                    href: "https://github.com/ki-editor/ki-editor",
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
                            label: "Zulip",
                            href: "https://ki-editor.zulipchat.com/join/zzhagqzl6wyzpqfeqxcsrkin/",
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
                            label: "GitHub",
                            href: "https://github.com/ki-editor/ki-editor",
                        },
                    ],
                },
            ],
            copyright: `Copyright © ${new Date().getFullYear()} Ki Editor. Built with Docusaurus.`,
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
