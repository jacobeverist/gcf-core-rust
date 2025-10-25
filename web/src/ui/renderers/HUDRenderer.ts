// Heads-up display renderer for simple metrics (FPS, steps, blocks, anomaly, predicted class)

export class HUDRenderer {
  constructor(private root: Document | HTMLElement = document) {}

  setStep(n: number) { this.setText('#step-counter', String(n)); }
  setFPS(fps: number) { this.setText('#fps-counter', Number.isFinite(fps) ? fps.toFixed(0) : '0'); }
  setBlocks(n: number) { this.setText('#block-counter', String(n)); }

  setAnomaly(v?: number) {
    const metric = this.qs('#anomaly-metric');
    if (!metric) return;
    if (typeof v === 'number') {
      metric.setAttribute('style', '');
      this.setText('#anomaly-value', v.toFixed(2));
    } else {
      metric.setAttribute('style', 'display: none;');
    }
  }

  setPredictedClass(name?: string) {
    const metric = this.qs('#class-metric');
    if (!metric) return;
    if (typeof name === 'string' && name.length > 0) {
      metric.setAttribute('style', '');
      this.setText('#class-value', name);
    } else {
      metric.setAttribute('style', 'display: none;');
    }
  }

  private setText(sel: string, text: string) {
    const el = this.qs(sel);
    if (el) el.textContent = text;
  }

  private qs<T extends Element = Element>(sel: string): T | null {
    const rootEl = this.root instanceof Document ? this.root : (this.root as HTMLElement);
    return rootEl.querySelector(sel) as T | null;
  }
}
