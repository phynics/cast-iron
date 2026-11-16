import { inflateSync } from "node:zlib";

/** Decodes a non-interlaced 8-bit RGB or RGBA PNG, as Playwright writes screenshots. */
function decode(png: Buffer) {
  let at = 8;
  const data: Buffer[] = [];
  let width = 0;
  let height = 0;
  let colorType = 0;
  while (at < png.length) {
    const length = png.readUInt32BE(at);
    const type = png.toString("latin1", at + 4, at + 8);
    const body = png.subarray(at + 8, at + 8 + length);
    if (type === "IHDR") {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      colorType = body[9];
    }
    if (type === "IDAT") data.push(body);
    at += 12 + length;
  }
  const bpp = colorType === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(data));
  const stride = width * bpp;
  const pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    for (let x = 0; x < stride; x++) {
      const value = raw[y * (stride + 1) + 1 + x];
      const left = x >= bpp ? pixels[y * stride + x - bpp] : 0;
      const up = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const upLeft = x >= bpp && y > 0 ? pixels[(y - 1) * stride + x - bpp] : 0;
      let out: number;
      if (filter === 0) out = value;
      else if (filter === 1) out = value + left;
      else if (filter === 2) out = value + up;
      else if (filter === 3) out = value + ((left + up) >> 1);
      else {
        const guess = left + up - upLeft;
        const [dl, du, dul] = [Math.abs(guess - left), Math.abs(guess - up), Math.abs(guess - upLeft)];
        out = value + (dl <= du && dl <= dul ? left : du <= dul ? up : upLeft);
      }
      pixels[y * stride + x] = out & 255;
    }
  }
  return { width, height, bpp, pixels };
}

/** How many pixels differ between two PNGs of one size; the size itself differing counts every pixel. */
export function differingPixels(a: Buffer, b: Buffer): number {
  const [pa, pb] = [decode(a), decode(b)];
  if (pa.width !== pb.width || pa.height !== pb.height) return Math.max(pa.width * pa.height, pb.width * pb.height);
  let count = 0;
  for (let i = 0; i < pa.width * pa.height; i++) {
    const k = i * pa.bpp;
    if (pa.pixels[k] !== pb.pixels[k] || pa.pixels[k + 1] !== pb.pixels[k + 1] || pa.pixels[k + 2] !== pb.pixels[k + 2]) count++;
  }
  return count;
}
