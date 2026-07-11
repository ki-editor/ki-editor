import { useRef, useEffect } from "react";
import BrowserOnly from "@docusaurus/BrowserOnly";
import type { Options } from "asciinema-player";
import "asciinema-player/dist/bundle/asciinema-player.css";

type Props = {
    src: string;
} & Options;

function Player({ src, ...opts }: Props) {
    const ref = useRef<HTMLDivElement>(null);
    const playerRef = useRef<{ dispose(): void } | null>(null);

    useEffect(() => {
        if (!ref.current) return;
        let cancelled = false;
        import("asciinema-player").then(({ create }) => {
            if (cancelled || !ref.current) return;
            playerRef.current = create(src, ref.current!, opts);
        });
        return () => {
            cancelled = true;
            playerRef.current?.dispose();
            playerRef.current = null;
        };
    }, [src]);

    return <div ref={ref} />;
}

export default function AsciinemaPlayer(props: Props) {
    return (
        <BrowserOnly fallback={<div style={{ minHeight: 200 }} />}>
            {() => <Player {...props} />}
        </BrowserOnly>
    );
}
