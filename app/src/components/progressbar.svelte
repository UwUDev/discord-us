<script lang="ts">
    export let ranges: [number, number][] = [];
    export let total: number;


    export let canvas: HTMLCanvasElement;

    $: {
        if (canvas) {

            const r = ranges.sort((a, b) => a[0] - b[0]);
            let cursor = 0;

            const ctx = canvas.getContext('2d');

            const width = canvas.width;
            const height = canvas.height;


            ctx.clearRect(0, 0, width, height);

            let part = total / (width);
            let step = Math.floor(part / 2);

            let incr = 5;

            const sX = (width / 200)
            const sY = (height / 17);

            const inBoundStart = (x: number) => cursor < r.length && x + step >= r[cursor][0];
            const inBoundEnd = (x: number) => cursor < r.length && Math.min(x + step, total) <= r[cursor][1];

            for (let i = 0; i < width; i += incr) {
                const start = Math.max(i, width + sX);
                const end = Math.min(i, width - sX);

                const b = i * part;

                if (cursor < r.length) {
                    if (inBoundStart(b)) {
                        if (inBoundEnd(b)) {
                            ctx.fillStyle = "#1e90ff";
                        } else {
                            cursor++;
                            if (inBoundEnd(b)) {
                                ctx.fillStyle = "#1e90ff";
                            }
                        }

                    } else {
                        cursor++;
                        if (inBoundStart(b)) {
                            i -= incr;
                            continue;
                        } else {
                            ctx.fillStyle = '#bbb';
                            cursor--;
                        }
                    }
                } else {
                    ctx.fillStyle = '#bbb';
                }

                ctx.fillRect(start, sY, end - start, height - 2*sY);
            }

            // draw frame
            ctx.fillStyle = '#000';
            ctx.fillRect(0, sY, sX, height - 2 * sY);

            ctx.fillRect(sX, 0, width - 2 * sX, sY);

            ctx.fillRect(sX, height - sY, width - 2 * sX, sY);

            ctx.fillRect(width - sX, sY, sX, height - 2 * sY);
        }
    }
</script>

<canvas bind:this={canvas}></canvas>