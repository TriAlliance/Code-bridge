// Code Bridge Web UI Application

const API_URL = window.location.hostname === 'localhost'
    ? 'http://localhost:8080'
    : `//${window.location.hostname}:8080`;

// State
let currentView = 'files';
let files = [];
let screenshots = [];
let peers = [];

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    initNavigation();
    initSearch();
    loadFiles();
    checkConnection();
});

// Navigation
function initNavigation() {
    document.querySelectorAll('.nav-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const view = btn.dataset.view;
            switchView(view);
        });
    });
}

function switchView(view) {
    currentView = view;

    // Update nav buttons
    document.querySelectorAll('.nav-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.view === view);
    });

    // Update views
    document.querySelectorAll('.view').forEach(v => {
        v.classList.toggle('active', v.id === `${view}-view`);
    });

    // Load data for view
    switch (view) {
        case 'files':
            loadFiles();
            break;
        case 'screenshots':
            loadScreenshots();
            break;
        case 'peers':
            loadPeers();
            break;
    }
}

// Search
function initSearch() {
    document.getElementById('file-search')?.addEventListener('input', (e) => {
        filterFiles(e.target.value);
    });

    document.getElementById('screenshot-search')?.addEventListener('input', (e) => {
        filterScreenshots(e.target.value);
    });

    document.getElementById('refresh-files')?.addEventListener('click', loadFiles);
    document.getElementById('refresh-screenshots')?.addEventListener('click', loadScreenshots);
    document.getElementById('discover-peers')?.addEventListener('click', discoverPeers);
    document.getElementById('save-settings')?.addEventListener('click', saveSettings);
}

// API Functions
async function fetchAPI(endpoint, options = {}) {
    try {
        const response = await fetch(`${API_URL}${endpoint}`, {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                ...options.headers
            }
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        return await response.json();
    } catch (error) {
        console.error(`API Error (${endpoint}):`, error);
        return null;
    }
}

// File Functions
async function loadFiles() {
    const container = document.getElementById('file-list');
    container.innerHTML = '<div class="loading">Loading files...</div>';

    // For demo, show sample data
    // In production, this would call the API
    files = [
        { name: 'src/main.rs', size: 4250, modified: '2025-12-20T10:30:00Z', hash: 'abc123' },
        { name: 'src/lib.rs', size: 8120, modified: '2025-12-20T09:15:00Z', hash: 'def456' },
        { name: 'Cargo.toml', size: 1024, modified: '2025-12-19T14:20:00Z', hash: 'ghi789' },
    ];

    renderFiles(files);
}

function renderFiles(filesToRender) {
    const container = document.getElementById('file-list');

    if (filesToRender.length === 0) {
        container.innerHTML = '<div class="loading">No files found</div>';
        return;
    }

    container.innerHTML = filesToRender.map(file => `
        <div class="file-item">
            <span class="file-icon">${getFileIcon(file.name)}</span>
            <div class="file-info">
                <div class="file-name">${file.name}</div>
                <div class="file-meta">
                    ${formatSize(file.size)} • ${formatDate(file.modified)}
                </div>
            </div>
            <div class="file-actions">
                <button onclick="downloadFile('${file.hash}')">Download</button>
                <button onclick="shareFile('${file.hash}')">Share</button>
            </div>
        </div>
    `).join('');
}

function filterFiles(query) {
    const filtered = files.filter(f =>
        f.name.toLowerCase().includes(query.toLowerCase())
    );
    renderFiles(filtered);
}

function getFileIcon(name) {
    const ext = name.split('.').pop().toLowerCase();
    const icons = {
        'rs': '🦀',
        'js': '📜',
        'ts': '📘',
        'py': '🐍',
        'swift': '🍎',
        'json': '📋',
        'toml': '⚙️',
        'md': '📝',
        'png': '🖼️',
        'jpg': '🖼️',
        'jpeg': '🖼️',
    };
    return icons[ext] || '📄';
}

// Screenshot Functions
async function loadScreenshots() {
    const container = document.getElementById('screenshot-grid');
    container.innerHTML = '<div class="loading">Loading screenshots...</div>';

    // Demo data
    screenshots = [
        { name: 'screenshot-001.png', date: '2025-12-20T10:30:00Z', ocr: 'Error message text' },
        { name: 'screenshot-002.png', date: '2025-12-20T09:15:00Z', ocr: 'Dashboard view' },
        { name: 'screenshot-003.png', date: '2025-12-19T14:20:00Z', ocr: 'Settings panel' },
    ];

    renderScreenshots(screenshots);
}

function renderScreenshots(screenshotsToRender) {
    const container = document.getElementById('screenshot-grid');

    if (screenshotsToRender.length === 0) {
        container.innerHTML = '<div class="loading">No screenshots found</div>';
        return;
    }

    container.innerHTML = screenshotsToRender.map(s => `
        <div class="screenshot-card" onclick="viewScreenshot('${s.name}')">
            <div class="screenshot-thumb" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
            </div>
            <div class="screenshot-info">
                <div class="screenshot-name">${s.name}</div>
                <div class="screenshot-date">${formatDate(s.date)}</div>
            </div>
        </div>
    `).join('');
}

function filterScreenshots(query) {
    const filtered = screenshots.filter(s =>
        s.name.toLowerCase().includes(query.toLowerCase()) ||
        s.ocr.toLowerCase().includes(query.toLowerCase())
    );
    renderScreenshots(filtered);
}

// Peer Functions
async function loadPeers() {
    const container = document.getElementById('peer-list');
    container.innerHTML = '<div class="loading">Loading peers...</div>';

    // Demo data
    peers = [
        { name: 'Mac mini', id: '12D3KooWABC...', status: 'online' },
        { name: 'Ubuntu Workstation', id: '12D3KooWDEF...', status: 'online' },
    ];

    renderPeers(peers);
}

function renderPeers(peersToRender) {
    const container = document.getElementById('peer-list');

    if (peersToRender.length === 0) {
        container.innerHTML = '<div class="loading">No peers found. Click "Discover Peers" to search.</div>';
        return;
    }

    container.innerHTML = peersToRender.map(p => `
        <div class="peer-card">
            <div class="peer-status ${p.status === 'online' ? '' : 'offline'}"></div>
            <div class="peer-info">
                <div class="peer-name">${p.name}</div>
                <div class="peer-id">${p.id}</div>
            </div>
            <button onclick="syncWithPeer('${p.id}')">Sync</button>
        </div>
    `).join('');
}

async function discoverPeers() {
    const container = document.getElementById('peer-list');
    container.innerHTML = '<div class="loading">Discovering peers...</div>';

    // Simulate discovery
    setTimeout(() => {
        loadPeers();
    }, 2000);
}

// Settings
async function saveSettings() {
    const settings = {
        deviceName: document.getElementById('device-name').value,
        watchDirs: document.getElementById('watch-dirs').value.split('\n').filter(d => d.trim()),
        ignorePatterns: document.getElementById('ignore-patterns').value.split('\n').filter(p => p.trim()),
    };

    console.log('Saving settings:', settings);
    alert('Settings saved!');
}

// Utility Functions
function formatSize(bytes) {
    const units = ['B', 'KB', 'MB', 'GB'];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
        size /= 1024;
        unitIndex++;
    }

    return `${size.toFixed(1)} ${units[unitIndex]}`;
}

function formatDate(dateStr) {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now - date;

    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;

    return date.toLocaleDateString();
}

// Action Functions
function downloadFile(hash) {
    console.log('Downloading:', hash);
    alert(`Downloading file ${hash}...`);
}

function shareFile(hash) {
    console.log('Sharing:', hash);
    alert(`Sharing file ${hash} with peers...`);
}

function viewScreenshot(name) {
    console.log('Viewing:', name);
    alert(`Opening ${name}...`);
}

function syncWithPeer(peerId) {
    console.log('Syncing with:', peerId);
    alert(`Syncing with peer ${peerId}...`);
}

// Connection Status
async function checkConnection() {
    const status = document.getElementById('status');

    try {
        const response = await fetch(`${API_URL}/health`, { timeout: 5000 });
        if (response.ok) {
            status.textContent = 'Connected';
            status.classList.add('connected');
        } else {
            throw new Error('Not connected');
        }
    } catch {
        status.textContent = 'Offline';
        status.classList.remove('connected');
    }

    // Check again in 30 seconds
    setTimeout(checkConnection, 30000);
}
