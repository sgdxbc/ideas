document.addEventListener('DOMContentLoaded', () => {
    for (let node of document.querySelectorAll('pre')) {
        node.classList.add('highlight');
        const wrapped = document.createElement('div');
        wrapped.classList.add('highlight');
        node.parentElement.insertBefore(wrapped, node);
        wrapped.appendChild(node);
    }
});