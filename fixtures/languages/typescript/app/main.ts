import { Job } from '@core/job';
export function schedule(job: Job): string { return job.run('queued'); }
export const unknownModule = (name: string) => import(name);
