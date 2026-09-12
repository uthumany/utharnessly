import test from 'node:test';
import assert from 'node:assert/strict';

test('local commands keep multiword arguments and reject unknown slash commands', async () => {
  const implementation = await import('../src/commands.js').catch(() => null);
  assert.ok(implementation, 'Slash commands need a deterministic local route');
  assert.deepEqual(implementation.localCommandArgs('/memory find two words'), ['memory', 'search', 'find two words']);
  assert.deepEqual(implementation.localCommandArgs('/new my session'), ['sessions', 'new', 'my session']);
  assert.deepEqual(implementation.localCommandArgs('/doctor'), ['doctor']);
  assert.deepEqual(implementation.localCommandArgs('/tools'), ['tools']);
  assert.throws(() => implementation.localCommandArgs('/not-a-command'), /Unknown command/);
});
