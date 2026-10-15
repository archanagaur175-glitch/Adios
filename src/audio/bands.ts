/** Pure analyser → band util: bass 20–150 Hz, mid 150–2 kHz,
    high 2–12 kHz (normalised 0–1) plus time-domain RMS. */

export interface Bands {
  bass: number;
  mid: number;
  high: number;
  rms: number;
}

let freqData: Uint8Array<ArrayBuffer> | null = null;
let timeData: Uint8Array<ArrayBuffer> | null = null;

export function readBands(analyser: AnalyserNode): Bands {
  if (!freqData || freqData.length !== analyser.frequencyBinCount) {
    freqData = new Uint8Array(analyser.frequencyBinCount);
  }
  if (!timeData || timeData.length !== analyser.fftSize) {
    timeData = new Uint8Array(analyser.fftSize);
  }
  analyser.getByteFrequencyData(freqData);
  analyser.getByteTimeDomainData(timeData);

  const nyquist = analyser.context.sampleRate / 2;
  const bin = (hz: number) =>
    Math.min(freqData!.length - 1, Math.round((hz / nyquist) * freqData!.length));
  const bandAvg = (fromHz: number, toHz: number) => {
    const from = bin(fromHz);
    const to = bin(toHz);
    let sum = 0;
    let count = 0;
    for (let i = from; i <= to; i++) {
      sum += freqData![i];
      count += 1;
    }
    return count > 0 ? sum / count / 255 : 0;
  };

  let squares = 0;
  for (let i = 0; i < timeData.length; i++) {
    const v = (timeData[i] - 128) / 128;
    squares += v * v;
  }
  const rms = Math.sqrt(squares / timeData.length);

  return {
    bass: bandAvg(20, 150),
    mid: bandAvg(150, 2000),
    high: bandAvg(2000, 12000),
    rms,
  };
}
