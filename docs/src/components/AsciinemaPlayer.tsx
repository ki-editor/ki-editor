import BrowserOnly from "@docusaurus/BrowserOnly";
import type { Options } from "asciinema-player";
import { useEffect, useRef } from "react";
import "asciinema-player/dist/bundle/asciinema-player.css";

type Props = {
    src: string;
} & Options;

function Player({ src, ...opts }: Props) {
    const ref = useRef<HTMLDivElement>(null);
    const playerRef = useRef<{ dispose(): void } | null>(null);

    // Stringify the options object to give useEffect a stable dependency primitive.
    // This stops the infinite re-render loop without requiring a complex useMemo setup.
    const serializedOpts = JSON.stringify(opts);

    useEffect(() => {
        const container = ref.current;
        if (!container) return;

        let cancelled = false;
        const parsedOpts = JSON.parse(serializedOpts) as Options;

        import("asciinema-player").then(({ create }) => {
            if (cancelled) return;
            // Capturing `container` locally satisfies TypeScript's control flow
            // analysis so we can drop the forbidden non-null (!) assertion.
            playerRef.current = create(src, container, parsedOpts);
        });

        return () => {
            cancelled = true;
            playerRef.current?.dispose();
            playerRef.current = null;
        };
    }, [src, serializedOpts]);

    return <div ref={ref} />;
}

export default function AsciinemaPlayer(props: Props) {
    return (
        <BrowserOnly fallback={<div style={{ minHeight: 200 }} />}>
            {() => <Player {...props} />}
        </BrowserOnly>
    );
}
