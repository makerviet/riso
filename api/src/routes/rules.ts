import { Hono } from 'hono'
import { jsonError, jsonSuccess } from '../responses'
import type { Bindings } from '../types'

type GameIndex = { games: { id: string; name: string; year: number; series: string; simulated: boolean }[] }

const app = new Hono<{ Bindings: Bindings }>()

async function readAsset(c: { env: Bindings; req: { url: string } }, path: string) {
  const url = new URL(c.req.url)
  url.pathname = `/${path}`
  url.search = ''
  return c.env.PACK_ASSETS.fetch(new Request(url))
}

async function readIndex(c: { env: Bindings; req: { url: string } }): Promise<GameIndex> {
  const response = await readAsset(c, 'index.json')
  if (!response.ok) throw new Error('index.json is unavailable')
  return response.json<GameIndex>()
}

app.get('/', async (c) => {
  try {
    return jsonSuccess(c, await readIndex(c))
  } catch (error) {
    return jsonError(c, 503, 'INTERNAL_ERROR', error instanceof Error ? error.message : 'Rules index is unavailable.')
  }
})

app.get('/:id', async (c) => {
  const id = c.req.param('id')
  try {
    const index = await readIndex(c)
    if (!index.games.some((game) => game.id === id)) return c.text('Không có đề này.', 404)
    const response = await readAsset(c, `${id}/rules.yaml`)
    if (!response.ok) return c.text('Đề chưa có rules.yaml.', 404)
    return new Response(response.body, {
      headers: { 'content-type': 'text/yaml; charset=utf-8', 'cache-control': 'public, max-age=300' },
    })
  } catch {
    return c.text('Dịch vụ luật đang lỗi.', 503)
  }
})

export default app
