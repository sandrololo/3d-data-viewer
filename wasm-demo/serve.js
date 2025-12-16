#!/usr/bin/env node
/**
 * Simple HTTP server for the WASM demo
 * 
 * Usage: node serve.js [port]
 * Default port: 8080
 */

var http = require('http');
var fs = require('fs');
var path = require('path');

var PORT = process.argv[2] || 8080;
var ROOT_DIR = __dirname;

var MIME_TYPES = {
    '.html': 'text/html',
    '.js': 'application/javascript',
    '.mjs': 'application/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.wasm': 'application/wasm',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.gif': 'image/gif',
    '.svg': 'image/svg+xml',
    '.ico': 'image/x-icon',
    '.tiff': 'image/tiff',
    '.tif': 'image/tiff'
};

function getMimeType(filePath) {
    var ext = path.extname(filePath).toLowerCase();
    return MIME_TYPES[ext] || 'application/octet-stream';
}

function handleRequest(req, res) {
    // Parse URL and remove query string
    var urlPath = req.url.split('?')[0];

    // Default to index.html
    if (urlPath === '/') {
        urlPath = '/index.html';
    }

    // Construct file path
    var filePath = path.join(ROOT_DIR, urlPath);

    // Security: prevent directory traversal
    if (filePath.indexOf(ROOT_DIR) !== 0) {
        res.writeHead(403);
        res.end('Forbidden');
        return;
    }

    // Check if file exists
    fs.stat(filePath, function (err, stats) {
        if (err || !stats.isFile()) {
            res.writeHead(404);
            res.end('Not Found');
            console.log('404: ' + urlPath);
            return;
        }

        // Get MIME type
        var mimeType = getMimeType(filePath);

        // Set headers
        res.setHeader('Content-Type', mimeType);
        res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
        res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');

        // Stream the file
        var stream = fs.createReadStream(filePath);
        stream.pipe(res);

        console.log('200: ' + urlPath + ' (' + mimeType + ')');
    });
}

var server = http.createServer(handleRequest);

server.listen(PORT, function () {
    console.log('');
    console.log('╔════════════════════════════════════════════════════════════╗');
    console.log('║                                                            ║');
    console.log('║   🦀 3D Data Viewer - WebAssembly Demo Server              ║');
    console.log('║                                                            ║');
    console.log('║   Server running at:                                       ║');
    console.log('║   http://localhost:' + PORT + '/                                     ║');
    console.log('║                                                            ║');
    console.log('║   Press Ctrl+C to stop                                     ║');
    console.log('║                                                            ║');
    console.log('╚════════════════════════════════════════════════════════════╝');
    console.log('');
});
