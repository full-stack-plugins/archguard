export class Job {
  run(value: string): string;
  run(value: number): number;
  run(value: string | number): string | number { return value; }
}
