// Custom JavaScript for MockForge documentation
// Add your custom JavaScript here

console.log('MockForge documentation loaded');

// Load Mermaid for diagram rendering
document.addEventListener('DOMContentLoaded', function() {
    console.log('Loading Mermaid...');

    // Load Mermaid from CDN
    const script = document.createElement('script');
    script.src = 'https://cdn.jsdelivr.net/npm/mermaid@10.6.1/dist/mermaid.min.js';
    script.onload = function() {
        console.log('Mermaid loaded successfully');

        try {
            // Initialize Mermaid
            mermaid.initialize({
                startOnLoad: true,
                theme: 'default',
                securityLevel: 'loose',
                fontFamily: 'arial',
                fontSize: 14,
                themeVariables: {
                    fontFamily: 'arial',
                    fontSize: '14px'
                }
            });

            console.log('Mermaid initialized');

            // Find all code blocks with language-mermaid class and render them
            const mermaidBlocks = document.querySelectorAll('code.language-mermaid');
            console.log('Found', mermaidBlocks.length, 'Mermaid code blocks');

            mermaidBlocks.forEach(function(block, index) {
                const code = block.textContent.trim();
                console.log('Processing Mermaid block', index + 1, 'with code length:', code.length);

                const container = document.createElement('div');
                container.className = 'mermaid';
                container.id = 'mermaid-diagram-' + index;
                container.textContent = code;

                // Replace the code block with the mermaid container
                block.parentNode.replaceWith(container);
            });

            // Render all mermaid diagrams
            setTimeout(function() {
                mermaid.init(undefined, document.querySelectorAll('.mermaid'))
                    .then(function() {
                        console.log('Mermaid diagrams rendered successfully');
                    })
                    .catch(function(error) {
                        console.error('Error rendering Mermaid diagrams:', error);
                    });
            }, 100);

        } catch (error) {
            console.error('Error initializing Mermaid:', error);
        }
    };

    script.onerror = function() {
        console.error('Failed to load Mermaid from CDN');
    };

    document.head.appendChild(script);
});
