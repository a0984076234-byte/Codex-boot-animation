import AppKit
let directory = CommandLine.arguments[1]
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = size * scale
        let image = NSImage(size: NSSize(width: pixels, height: pixels))
        image.lockFocus()
        let edge = CGFloat(pixels)
        let box = NSRect(x: edge * 0.06, y: edge * 0.06, width: edge * 0.88, height: edge * 0.88)
        let path = NSBezierPath(roundedRect: box, xRadius: edge * 0.20, yRadius: edge * 0.20)
        NSGradient(starting: NSColor(calibratedRed: 0.15, green: 0.22, blue: 0.26, alpha: 1), ending: NSColor(calibratedRed: 0.035, green: 0.055, blue: 0.07, alpha: 1))!.draw(in: path, angle: -90)
        let text = "🐉" as NSString
        let attributes: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: edge * 0.64)]
        let bounds = text.size(withAttributes: attributes)
        text.draw(at: NSPoint(x: (edge - bounds.width) / 2, y: (edge - bounds.height) / 2), withAttributes: attributes)
        image.unlockFocus()
        let bitmap = NSBitmapImageRep(data: image.tiffRepresentation!)!
        let name = "icon_\(size)x\(size)\(scale == 2 ? "@2x" : "").png"
        try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: directory).appendingPathComponent(name))
    }
}
// Write the standard PNG-backed ICNS container without relying on iconutil.
func uint32(_ value: Int) -> Data {
    var number = UInt32(value).bigEndian
    return withUnsafeBytes(of: &number) { Data($0) }
}
var chunks = Data()
for (type, file) in [("icp4", "icon_16x16.png"), ("icp5", "icon_32x32.png"), ("icp6", "icon_32x32@2x.png"), ("ic07", "icon_128x128.png"), ("ic08", "icon_256x256.png"), ("ic09", "icon_512x512.png"), ("ic10", "icon_512x512@2x.png")] {
    let png = try Data(contentsOf: URL(fileURLWithPath: directory).appendingPathComponent(file))
    chunks.append(Data(type.utf8))
    chunks.append(uint32(png.count + 8))
    chunks.append(png)
}
var icon = Data("icns".utf8)
icon.append(uint32(chunks.count + 8))
icon.append(chunks)
try icon.write(to: URL(fileURLWithPath: directory).appendingPathExtension("icns"))
