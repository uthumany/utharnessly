import test from 'node:test';
import assert from 'node:assert/strict';
import { PIPELINE_STAGES, pipelineProgress, pipelineStage } from '../src/tui/response-progress.js';

test('response pipeline covers the named lifecycle in a stable 0–100 order', () => {
  assert.deepEqual(PIPELINE_STAGES.map(stage => stage.label), [
    'Understanding', 'Decomposing', 'Retrieving', 'Grounding', 'Planning',
    'Reasoning', 'Routing', 'Executing', 'Observing', 'Evaluating',
    'Synthesizing', 'Verifying', 'Refining', 'Finalizing', 'Responding'
  ]);
  assert.equal(PIPELINE_STAGES[0]?.percent, 0);
  assert.equal(PIPELINE_STAGES.at(-1)?.percent, 100);
  assert.deepEqual([...PIPELINE_STAGES].map(stage => stage.percent).sort((a, b) => a - b), PIPELINE_STAGES.map(stage => stage.percent));
});

test('pipeline helper exposes the active stage without claiming model-token progress', () => {
  assert.equal(pipelineProgress('executing'), 49);
  assert.equal(pipelineStage('reasoning').activeLabel, 'Reasoning');
  assert.match(pipelineStage('reasoning').detail, /provider is working/i);
  assert.equal(pipelineStage('responding').complete, true);
});
