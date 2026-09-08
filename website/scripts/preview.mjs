// Keep the test server in the foreground, including agent environments where
// Astro's CLI automatically launches a detached background process.
import { preview } from 'astro';
const server = await preview({ server: { host: '127.0.0.1', port: 4325 } });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, async () => { await server.stop(); process.exit(0); });
