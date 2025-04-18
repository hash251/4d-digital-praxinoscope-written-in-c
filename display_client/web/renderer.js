window.api.receive('image-update', (imageSrc) => {
    console.log('Updating image source to:', imageSrc);
    const image = document.getElementById('image');
    if (image) {
        image.src = imageSrc;
    }
});

document.addEventListener('DOMContentLoaded', () => {
    console.log('Renderer process initialized, waiting for images...');
});
