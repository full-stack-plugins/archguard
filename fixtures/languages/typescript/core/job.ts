export class Job {
  run(input: string): string;
  run(input: number): number;
  run(input: string | number): string | number { return input; }
}
