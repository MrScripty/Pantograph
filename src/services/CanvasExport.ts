import { Logger } from './Logger';

class CanvasExportService {
  private canvasElement: HTMLCanvasElement | null = null;

  public setCanvas(canvas: HTMLCanvasElement): void {
    this.canvasElement = canvas;
    Logger.log('CANVAS_REGISTERED', { width: canvas.width, height: canvas.height });
  }

  public exportToBase64(): string | null {
    if (!this.canvasElement) {
      Logger.log('CANVAS_EXPORT_ERROR', { error: 'No canvas element registered' }, 'error');
      return null;
    }

    try {
      const dataUrl = this.canvasElement.toDataURL('image/png');
      const base64 = dataUrl.replace(/^data:image\/png;base64,/, '');

      Logger.log('CANVAS_EXPORTED', {
        width: this.canvasElement.width,
        height: this.canvasElement.height,
        base64Length: base64.length,
      });

      return base64;
    } catch (error) {
      Logger.log('CANVAS_EXPORT_ERROR', { error: String(error) }, 'error');
      return null;
    }
  }

  public exportToDataUrl(): string | null {
    if (!this.canvasElement) {
      return null;
    }
    return this.canvasElement.toDataURL('image/png');
  }
}

export const canvasExport = new CanvasExportService();
