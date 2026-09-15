export const PIPELINE_STAGES = [
  { id: 'understanding', label: 'Understanding', percent: 0, detail: 'Prompt accepted' },
  { id: 'decomposing', label: 'Decomposing', percent: 7, detail: 'Preparing the request' },
  { id: 'retrieving', label: 'Retrieving', percent: 14, detail: 'Loading relevant local context' },
  { id: 'grounding', label: 'Grounding', percent: 21, detail: 'Applying local instructions and safety rules' },
  { id: 'planning', label: 'Planning', percent: 28, detail: 'Building the provider request' },
  { id: 'reasoning', label: 'Reasoning', percent: 35, detail: 'The provider is working; token progress is not measurable' },
  { id: 'routing', label: 'Routing', percent: 42, detail: 'Selecting the configured provider and model' },
  { id: 'executing', label: 'Executing', percent: 49, detail: 'Request is running' },
  { id: 'observing', label: 'Observing', percent: 63, detail: 'Provider output received' },
  { id: 'evaluating', label: 'Evaluating', percent: 72, detail: 'Checking the provider result' },
  { id: 'synthesizing', label: 'Synthesizing', percent: 80, detail: 'Assembling the response' },
  { id: 'verifying', label: 'Verifying', percent: 88, detail: 'Validating completion' },
  { id: 'refining', label: 'Refining', percent: 94, detail: 'Normalizing response formatting' },
  { id: 'finalizing', label: 'Finalizing', percent: 98, detail: 'Persisting the completed turn' },
  { id: 'responding', label: 'Responding', percent: 100, detail: 'Response complete' }
] as const;

export type PipelineStageId = typeof PIPELINE_STAGES[number]['id'];
export type PipelineStage = typeof PIPELINE_STAGES[number] & { complete: boolean; activeLabel: string };

export function pipelineStage(id: PipelineStageId): PipelineStage {
  const stage = PIPELINE_STAGES.find(item => item.id === id);
  if (!stage) throw new Error(`Unknown response pipeline stage: ${id}`);
  return { ...stage, complete: stage.percent === 100, activeLabel: stage.label };
}

export function pipelineProgress(id: PipelineStageId): number {
  return pipelineStage(id).percent;
}

export function responseProgressBar(percent: number, width = 12, unicode = true): string {
  const safePercent = Math.max(0, Math.min(100, percent));
  const filled = Math.round((safePercent / 100) * width);
  return `${(unicode ? '█' : '#').repeat(filled)}${(unicode ? '░' : '-').repeat(width - filled)}`;
}
