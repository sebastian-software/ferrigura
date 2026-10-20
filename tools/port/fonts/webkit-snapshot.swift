// Snapshot of a page in the system WebKit: the engine of the Safari installed on this Mac, which
// Playwright's WebKit build is not. Used to check that Safari draws the bundled faces' colour emoji
// (docs-port/spikes/fallback-faces.md, section 8).
//
//   xcrun swiftc -O tools/port/fonts/webkit-snapshot.swift -o target/webkit-snapshot
//   target/webkit-snapshot <page.html> <snapshot.png>
//
// The page is loaded from a file with read access to its folder, so it can show an SVG both inline
// and through <img src>. The snapshot is taken two seconds after the page has loaded.

import AppKit
import WebKit

// Loads page.html in the system WebKit (the engine of the installed Safari) and writes snapshot.png.
final class Snapper: NSObject, WKNavigationDelegate {
    let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 1100, height: 700))
    func run() {
        view.navigationDelegate = self
        let url = URL(fileURLWithPath: CommandLine.arguments[1])
        view.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
    }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            webView.takeSnapshot(with: nil) { image, error in
                guard let image, let tiff = image.tiffRepresentation,
                      let rep = NSBitmapImageRep(data: tiff),
                      let png = rep.representation(using: .png, properties: [:]) else {
                    print("snapshot failed: \(String(describing: error))"); exit(1)
                }
                try? png.write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
                print("written")
                exit(0)
            }
        }
    }
}
let app = NSApplication.shared
let snapper = Snapper()
snapper.run()
app.run()
