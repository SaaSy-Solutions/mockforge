// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="index.html">Introduction</a></li><li class="chapter-item expanded "><a href="getting-started/installation.html"><strong aria-hidden="true">1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting-started/five-minute-api.html"><strong aria-hidden="true">2.</strong> Your First Mock API in 5 Minutes</a></li><li class="chapter-item expanded "><a href="getting-started/quick-start.html"><strong aria-hidden="true">3.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="getting-started/concepts.html"><strong aria-hidden="true">4.</strong> Basic Concepts</a></li><li class="chapter-item expanded "><a href="tutorials/index.html"><strong aria-hidden="true">5.</strong> Overview</a></li><li class="chapter-item expanded "><a href="tutorials/mock-openapi-spec.html"><strong aria-hidden="true">6.</strong> Mock a REST API from OpenAPI</a></li><li class="chapter-item expanded "><a href="tutorials/admin-ui-walkthrough.html"><strong aria-hidden="true">7.</strong> Admin UI Walkthrough</a></li><li class="chapter-item expanded "><a href="tutorials/add-custom-plugin.html"><strong aria-hidden="true">8.</strong> Add a Custom Plugin</a></li><li class="chapter-item expanded "><a href="user-guide/http-mocking.html"><strong aria-hidden="true">9.</strong> HTTP Mocking</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="user-guide/http-mocking/openapi.html"><strong aria-hidden="true">9.1.</strong> OpenAPI Integration</a></li><li class="chapter-item "><a href="user-guide/http-mocking/custom-responses.html"><strong aria-hidden="true">9.2.</strong> Custom Responses</a></li><li class="chapter-item "><a href="user-guide/http-mocking/dynamic-data.html"><strong aria-hidden="true">9.3.</strong> Dynamic Data</a></li></ol></li><li class="chapter-item expanded "><a href="user-guide/grpc-mocking.html"><strong aria-hidden="true">10.</strong> gRPC Mocking</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="user-guide/grpc-mocking/protobuf.html"><strong aria-hidden="true">10.1.</strong> Protocol Buffers</a></li><li class="chapter-item "><a href="user-guide/grpc-mocking/streaming.html"><strong aria-hidden="true">10.2.</strong> Streaming</a></li><li class="chapter-item "><a href="user-guide/grpc-mocking/advanced-data-synthesis.html"><strong aria-hidden="true">10.3.</strong> Advanced Data Synthesis</a></li></ol></li><li class="chapter-item expanded "><a href="user-guide/graphql-mocking.html"><strong aria-hidden="true">11.</strong> GraphQL Mocking</a></li><li class="chapter-item expanded "><a href="user-guide/websocket-mocking.html"><strong aria-hidden="true">12.</strong> WebSocket Mocking</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="user-guide/websocket-mocking/replay.html"><strong aria-hidden="true">12.1.</strong> Replay Mode</a></li><li class="chapter-item "><a href="user-guide/websocket-mocking/interactive.html"><strong aria-hidden="true">12.2.</strong> Interactive Mode</a></li></ol></li><li class="chapter-item expanded "><a href="user-guide/plugins.html"><strong aria-hidden="true">13.</strong> Plugin System</a></li><li class="chapter-item expanded "><a href="user-guide/security.html"><strong aria-hidden="true">14.</strong> Security &amp; Encryption</a></li><li class="chapter-item expanded "><a href="user-guide/sync.html"><strong aria-hidden="true">15.</strong> Directory Synchronization</a></li><li class="chapter-item expanded "><a href="user-guide/admin-ui.html"><strong aria-hidden="true">16.</strong> Admin UI</a></li><li class="chapter-item expanded "><a href="configuration/environment.html"><strong aria-hidden="true">17.</strong> Environment Variables</a></li><li class="chapter-item expanded "><a href="configuration/files.html"><strong aria-hidden="true">18.</strong> Configuration Files</a></li><li class="chapter-item expanded "><a href="configuration/advanced.html"><strong aria-hidden="true">19.</strong> Advanced Options</a></li><li class="chapter-item expanded "><a href="development/building.html"><strong aria-hidden="true">20.</strong> Building from Source</a></li><li class="chapter-item expanded "><a href="development/testing.html"><strong aria-hidden="true">21.</strong> Testing</a></li><li class="chapter-item expanded "><a href="development/architecture.html"><strong aria-hidden="true">22.</strong> Architecture</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="development/architecture/cli.html"><strong aria-hidden="true">22.1.</strong> CLI Crate</a></li><li class="chapter-item "><a href="development/architecture/http.html"><strong aria-hidden="true">22.2.</strong> HTTP Crate</a></li><li class="chapter-item "><a href="development/architecture/grpc.html"><strong aria-hidden="true">22.3.</strong> gRPC Crate</a></li><li class="chapter-item "><a href="development/architecture/ws.html"><strong aria-hidden="true">22.4.</strong> WebSocket Crate</a></li></ol></li><li class="chapter-item expanded "><a href="api/cli.html"><strong aria-hidden="true">23.</strong> CLI Reference</a></li><li class="chapter-item expanded "><a href="api/admin-ui-rest.html"><strong aria-hidden="true">24.</strong> Admin UI REST API</a></li><li class="chapter-item expanded "><a href="api/rust.html"><strong aria-hidden="true">25.</strong> Rust API</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="api/rust/http.html"><strong aria-hidden="true">25.1.</strong> HTTP Module</a></li><li class="chapter-item "><a href="api/rust/grpc.html"><strong aria-hidden="true">25.2.</strong> gRPC Module</a></li><li class="chapter-item "><a href="api/rust/ws.html"><strong aria-hidden="true">25.3.</strong> WebSocket Module</a></li></ol></li><li class="chapter-item expanded "><a href="contributing/setup.html"><strong aria-hidden="true">26.</strong> Development Setup</a></li><li class="chapter-item expanded "><a href="contributing/style.html"><strong aria-hidden="true">27.</strong> Code Style</a></li><li class="chapter-item expanded "><a href="contributing/testing.html"><strong aria-hidden="true">28.</strong> Testing Guidelines</a></li><li class="chapter-item expanded "><a href="contributing/release.html"><strong aria-hidden="true">29.</strong> Release Process</a></li><li class="chapter-item expanded "><a href="reference/config-schema.html"><strong aria-hidden="true">30.</strong> Configuration Schema</a></li><li class="chapter-item expanded "><a href="reference/config-validation.html"><strong aria-hidden="true">31.</strong> Configuration Validation</a></li><li class="chapter-item expanded "><a href="reference/formats.html"><strong aria-hidden="true">32.</strong> Supported Formats</a></li><li class="chapter-item expanded "><a href="reference/templating.html"><strong aria-hidden="true">33.</strong> Templating Reference</a></li><li class="chapter-item expanded "><a href="reference/chaining.html"><strong aria-hidden="true">34.</strong> Request Chaining</a></li><li class="chapter-item expanded "><a href="reference/fixtures.html"><strong aria-hidden="true">35.</strong> Fixtures and Smoke Testing</a></li><li class="chapter-item expanded "><a href="reference/troubleshooting.html"><strong aria-hidden="true">36.</strong> Troubleshooting</a></li><li class="chapter-item expanded "><a href="reference/faq.html"><strong aria-hidden="true">37.</strong> FAQ</a></li><li class="chapter-item expanded "><a href="reference/changelog.html"><strong aria-hidden="true">38.</strong> Changelog</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
