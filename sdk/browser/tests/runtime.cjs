const assert = require('node:assert/strict');
const {MockForgeClient} = require('../dist/index.js');
(async () => {
 const base=process.argv[2], client=new MockForgeClient(base, process.env.MOCKFORGE_MANAGEMENT_TOKEN);
 assert.equal(await client.healthCheck(),true);
 const created=await client.createMock({name:'SDK runtime control',method:'GET',path:'/sdk1281-browser',response:{body:{sdk:'browser'},headers:{'X-SDK-Control':'browser'}},status_code:202});
 assert.ok(created.id);
 assert.ok((await client.listMocks()).some(mock=>mock.id===created.id));
 assert.equal((await client.getMock(created.id)).id,created.id);
 let response=await fetch(base+'/sdk1281-browser');
 assert.equal(response.status,202);assert.deepEqual(await response.json(),{sdk:'browser'});assert.equal(response.headers.get('X-SDK-Control'),'browser');
 const updated=await client.updateMock(created.id,{method:'GET',path:'/sdk1281-browser',response:{body:{sdk:'browser',updated:true}},status_code:203});
 assert.equal(updated.id,created.id);assert.equal((await client.getMock(created.id)).id,created.id);
 response=await fetch(base+'/sdk1281-browser');assert.equal(response.status,203);assert.deepEqual(await response.json(),{sdk:'browser',updated:true});
 await client.deleteMock(created.id);
 await assert.rejects(()=>client.getMock(created.id),/404/);
 response=await fetch(base+'/sdk1281-browser');assert.equal(response.status,404);
 console.log('Browser actual CLI create/list/get/serve/update/delete control passed');
})().catch(error=>{console.error(error.message);process.exitCode=1;});
