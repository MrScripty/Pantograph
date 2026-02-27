import { WideEvent } from '../types';

class LoggerService {
  private readonly maxEvents: number;
  private readonly events: Array<WideEvent | undefined>;
  private writeIndex = 0;
  private size = 0;
  private droppedCount = 0;

  constructor(maxEvents = 5000) {
    this.maxEvents = Math.max(100, maxEvents);
    this.events = new Array<WideEvent | undefined>(this.maxEvents);
  }

  public log(type: string, payload: Record<string, unknown> = {}, severity: 'info' | 'warn' | 'error' = 'info') {
    const event: WideEvent = {
      timestamp: Date.now(),
      type,
      payload,
      severity,
    };

    if (this.size === this.maxEvents) {
      this.droppedCount++;
    } else {
      this.size++;
    }

    this.events[this.writeIndex] = event;
    this.writeIndex = (this.writeIndex + 1) % this.maxEvents;

    if (severity === 'error') {
      console.error(`[WIDE-EVENT][${severity.toUpperCase()}] ${type}`, payload);
    } else {
      console.log(`[WIDE-EVENT][${severity.toUpperCase()}] ${type}`, payload);
    }
  }

  public getEvents() {
    if (this.size === 0) return [];

    const ordered: WideEvent[] = [];
    const start = (this.writeIndex - this.size + this.maxEvents) % this.maxEvents;
    for (let i = 0; i < this.size; i++) {
      const idx = (start + i) % this.maxEvents;
      const event = this.events[idx];
      if (event) {
        ordered.push(event);
      }
    }
    return ordered;
  }

  public getRetentionStats() {
    return {
      maxEvents: this.maxEvents,
      retainedEvents: this.size,
      droppedEvents: this.droppedCount,
    };
  }
}

export const Logger = new LoggerService();
