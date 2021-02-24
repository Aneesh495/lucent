const encoder = new TextEncoder();

function crc(bytes: Uint8Array): number {
  let value = 0xffffffff;
  for (const byte of bytes) {
    value ^= byte;
    for (let j = 0; j < 8; j++) value = (value >>> 1) ^ ((value & 1) ? 0xedb88320 : 0);
  }
  return (value ^ 0xffffffff) >>> 0;
}

export async function annotatePng(image: Blob, metadata: unknown): Promise<Blob> {
  const bytes = new Uint8Array(await image.arrayBuffer());
  const size = bytes.length;
  const end = size - 12;
  if (size < 20 || bytes[0] !== 137 || bytes[1] !== 80 || bytes[2] !== 78 || bytes[3] !== 71
    || new DataView(bytes.buffer, end).getUint32(4) !== 0x49454e44) {
    throw Error('Canvas returned an invalid PNG');
  }
  const label = encoder.encode('Lucent');
  const value = encoder.encode(JSON.stringify(metadata));
  if (value.length > 32 * 1024 * 1024) throw Error('Export metadata exceeds 32 MiB');
  const dataLength = label.length + 5 + value.length;
  const chunk = new Uint8Array(dataLength + 12);
  const writer = new DataView(chunk.buffer);
  writer.setUint32(0, dataLength);
  writer.setUint32(4, 0x69545874);
  chunk.set(label, 8);
  // Separator, compression flag, compression method, language tag and translated keyword.
  chunk.set(value, 8 + label.length + 5);
  writer.setUint32(chunk.length - 4, crc(chunk.subarray(4, chunk.length - 4)));
  return new Blob([bytes.subarray(0, end), chunk, bytes.subarray(end)], { type: 'image/png' });
}
