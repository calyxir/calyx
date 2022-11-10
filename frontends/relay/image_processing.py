import numpy as np

import imageio
from PIL import Image


def get_image(path):
    """
    source: https://github.com/onnx/models/tree/main/vision/classification/inception_and_googlenet/googlenet
    Using path to image, return the RGB load image
    """
    img = imageio.imread(path, pilmode="RGB")
    return img


def preprocess_google(img_path):
    """
    source: https://github.com/onnx/models/tree/main/vision/classification/inception_and_googlenet/googlenet
    Preprocessing required on the images for inference with mxnet gluon
    The function takes loaded image and returns processed tensor
    """
    img = get_image(img_path)
    img = np.array(Image.fromarray(img).resize((224, 224))).astype(np.float32)
    img[:, :, 0] -= 123.68
    img[:, :, 1] -= 116.779
    img[:, :, 2] -= 103.939
    img[:, :, [0, 1, 2]] = img[:, :, [2, 1, 0]]
    img = img.transpose((2, 0, 1))
    img = np.expand_dims(img, axis=0)
    return img


def preprocess_img_mnist(img_path):
    """Preprocessing required for MNIST classification."""
    from PIL import Image
    import cv2

    img = Image.open(img_path)
    img = np.array(img)
    # Convert to grayscale.
    try:
        # This may cause an error if the image is already in grayscale.
        img = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    except:
        pass
    # Resize.
    img = cv2.resize(img, (28, 28)).astype(np.float32) / 255
    # Batchify.
    return np.reshape(img, (1, 1, 28, 28))


def preprocess_img_imagenet(img_path):
    """Preprocessing required for ImageNet classification.
    Reference:
      https://github.com/onnx/models/tree/master/vision/classification/vgg
    """
    import mxnet
    from mxnet.gluon.data.vision import transforms
    from PIL import Image

    img = Image.open(img_path)
    img = mxnet.ndarray.array(img)

    transform_fn = transforms.Compose(
        [
            transforms.Resize(224),
            transforms.CenterCrop(224),
            transforms.ToTensor(),
            transforms.Normalize([0.485, 0.456, 0.406], [0.229, 0.224, 0.225]),
        ]
    )
    img = transform_fn(img)
    img = img.expand_dims(axis=0)  # Batchify.
    return img.asnumpy()


# Supported datasets for preprocessing.
SupportedDatasets = {
    "mnist": preprocess_img_mnist,
    "imagenet": preprocess_img_imagenet,
    "googlenet": preprocess_google,
}


def preprocess_image(img, dataset: str):
    """Preprocesses an image for classification."""
    dataset = dataset.lower()
    if dataset not in SupportedDatasets.keys():
        raise Exception(
            f"Preprocessing the image for: {dataset} is not supported. "
            f"Supported datasets: {SupportedDatasets}"
        )
    return SupportedDatasets[dataset](img)
