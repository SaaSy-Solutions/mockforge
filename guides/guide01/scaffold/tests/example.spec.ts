import { test, expect } from '@playwright/test';
import { execSync } from 'node:child_process';
function exec(cmd){ execSync(cmd,{stdio:'inherit'}) }
test.beforeAll(() => { exec('mockforge up --background --port 4000') })
test.afterAll(() => { exec('mockforge down') })
test('hello route', async ({ request }) => {
  const res = await request.get('http://localhost:4000/api/hello?name=Ray')
  expect(await res.json()).toEqual({ message: 'Hello Ray' })
})
