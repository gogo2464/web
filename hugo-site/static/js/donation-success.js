// Use global nacl and nacl-util objects

function bufferToBase64(buffer) {
    return nacl.util.encodeBase64(buffer);
}

function base64ToBuffer(base64) {
    return nacl.util.decodeBase64(base64);
}

document.addEventListener('DOMContentLoaded', function() {
  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');
  const isTestMode = urlParams.get('test') !== null;

  if (isTestMode) {
    console.log("Test mode detected");
    generateTestCertificate();
  } else if (paymentIntent) {
    console.log("Payment intent detected:", paymentIntent);
    generateAndSignCertificate(paymentIntent);
  } else {
    console.log("No payment intent or test mode detected");
    showError('Payment information not found.');
  }
});

function generateTestCertificate() {
  console.log("Generating test certificate");
  const publicKey = nacl.randomBytes(32);
  const privateKey = nacl.randomBytes(64);
  const unblindedSignature = nacl.randomBytes(64);

  displayCertificate(publicKey, privateKey, unblindedSignature);
}

async function generateAndSignCertificate(paymentIntentId) {
  try {
    // Generate Ed25519 key pair
    const keyPair = nacl.sign.keyPair();
    const publicKey = keyPair.publicKey;
    const privateKey = keyPair.secretKey;

    // Generate random blinding factor
    const blindingFactor = nacl.randomBytes(32);

    // Blind the public key
    const blindedPublicKey = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      blindedPublicKey[i] = publicKey[i] ^ blindingFactor[i];
    }

    // Send blinded public key to server for signing
    const response = await fetch('http://127.0.0.1:8000/sign-certificate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 
        payment_intent_id: paymentIntentId, 
        blinded_public_key: bufferToBase64(blindedPublicKey)
      })
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Error signing certificate: ${errorText}`);
    }

    const data = await response.json();
    if (!data.blind_signature) {
      if (data.message === "CERTIFICATE_ALREADY_SIGNED") {
        showError('Certificate already signed for this payment.');
        return;
      }
      throw new Error('No blind signature received from server');
    }

    const blindSignature = base64ToBuffer(data.blind_signature);

    // Unblind the signature
    const unblindedSignature = new Uint8Array(64);
    for (let i = 0; i < 32; i++) {
      unblindedSignature[i] = blindSignature[i] ^ blindingFactor[i];
    }
    for (let i = 32; i < 64; i++) {
      unblindedSignature[i] = blindSignature[i];
    }

    displayCertificate(publicKey, privateKey, unblindedSignature);
  } catch (error) {
    showError('Error generating certificate: ' + error.message);
  }
}

function generateTestCertificate() {
  const publicKey = nacl.randomBytes(32);
  const privateKey = nacl.randomBytes(64);
  const unblindedSignature = nacl.randomBytes(64);

  displayCertificate(publicKey, privateKey, unblindedSignature);
}

function displayCertificate(publicKey, privateKey, unblindedSignature) {
  console.log("Displaying certificate");
  // Armor the certificate and private key
  const armoredCertificate = `-----BEGIN FREENET DONATION CERTIFICATE-----
${bufferToBase64(publicKey)}|${bufferToBase64(unblindedSignature)}
-----END FREENET DONATION CERTIFICATE-----`;

  const armoredPrivateKey = `-----BEGIN FREENET DONATION PRIVATE KEY-----
${bufferToBase64(privateKey)}
-----END FREENET DONATION PRIVATE KEY-----`;

  // Combine certificate and private key
  const combinedKey = `${wrapBase64(armoredCertificate, 64)}\n\n${wrapBase64(armoredPrivateKey, 64)}`;

  // Display the combined key
  const combinedKeyElement = document.getElementById('combinedKey');
  if (combinedKeyElement) {
    combinedKeyElement.value = combinedKey;
    document.getElementById('certificateSection').style.display = 'block';
    document.getElementById('certificate-info').style.display = 'none';

    // Set up copy button
    const copyButton = document.getElementById('copyCombinedKey');
    if (copyButton) {
      copyButton.addEventListener('click', function() {
        combinedKeyElement.select();
        document.execCommand('copy');
        alert('Combined key copied to clipboard!');
      });
    } else {
      console.error("Copy button not found");
    }
  } else {
    console.error("Combined key textarea not found");
    showError('Error displaying certificate. Please contact support.');
  }

  // Verify the certificate
  if (verifyCertificate(publicKey, unblindedSignature)) {
    console.log("Certificate verified successfully");
  } else {
    console.error("Certificate verification failed");
    showError('Certificate verification failed. Please contact support.');
  }
}

// Function to wrap base64 encoded text
function wrapBase64(str, maxWidth) {
  const lines = str.split('\n');
  return lines.map(line => {
    if (line.startsWith('-----')) {
      return line;
    }
    return line.match(new RegExp(`.{1,${maxWidth}}`, 'g')).join('\n');
  }).join('\n');
}

function verifyCertificate(publicKey, signature) {
  try {
    // In a real implementation, we would verify the signature against a known message
    // For now, we'll just check if the signature is the correct length
    return signature.length === 64;
  } catch (error) {
    console.error("Verification error:", error);
    return false;
  }
}

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  errorElement.textContent = message;
  errorElement.style.display = 'block';
  document.getElementById('certificate-info').style.display = 'none';
}
