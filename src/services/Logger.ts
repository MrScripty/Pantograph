import { WideEvent } from '../types';

class LoggerService {
  private events: WideEvent[] = [];

  public log(type: string, payload: any = {}, severity: 'info' | 'warn' | 'error' = 'info') {
    const event: WideEvent = {
      timestamp: Date.now(),
      type,
      payload,
      severity,
    };

    this.events.push(event);

    if (severity === 'error') {
      console.error(`[WIDE-EVENT][${severity.toUpperCase()}] ${type}`, payload);
    } else {
      console.log(`[WIDE-EVENT][${severity.toUpperCase()}] ${type}`, payload);
    }
  }

  public getEvents() {
    return [...this.events];
  }
}

export const Logger = new LoggerService();
