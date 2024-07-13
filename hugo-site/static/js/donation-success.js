// Use global nacl and nacl-util objects

function bufferToBase64(buffer) {
    return nacl.util.encodeBase64(buffer);
}

function base64ToBuffer(base64) {
    return nacl.util.decodeBase64(base64);
}

document.addEventListener('DOMContentLoaded', function() {
  console.log("DOM fully loaded");
  
  // Function to check for required elements
  function checkRequiredElements() {
    const requiredElements = [
      { id: 'certificateSection', selector: '#certificateSection' },
      { id: 'certificate-info', selector: '#certificate-info' },
      { id: 'errorMessage', selector: '#errorMessage' }
    ];
    
    const missingElements = requiredElements.filter(el => !document.querySelector(el.selector));
    
    if (missingElements.length > 0) {
      console.error("Missing required elements:", missingElements.map(el => el.id));
      showError(`Error: Missing required elements: ${missingElements.map(el => el.id).join(', ')}`);
      return false;
    }

    // Check for optional elements
    const optionalElements = [
      { id: 'combinedKey', selector: '#combinedKey' },
      { id: 'copyCombinedKey', selector: '#copyCombinedKey' }
    ];

    optionalElements.forEach(el => {
      if (!document.querySelector(el.selector)) {
        console.warn(`Optional element missing: ${el.id}`);
      }
    });

    return true;
  }

  // Function to initialize the page
  function initPage() {
    if (!checkRequiredElements()) {
      return;
    }

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
  }

  // Try to initialize immediately
  initPage();

  // If it fails, try again after a short delay
  setTimeout(initPage, 500);
});

function generateTestCertificate() {
  console.log("Generating test certificate");
  const publicKey = nacl.randomBytes(32);
  const privateKey = nacl.randomBytes(64);
  const unblindedSignature = nacl.randomBytes(64);

  displayCertificate(publicKey, privateKey, unblindedSignature);
}

async function generateAndSignCertificate(paymentIntentId) {
  console.log("Starting generateAndSignCertificate");
  try {
    // Generate Ed25519 key pair
    const keyPair = nacl.sign.keyPair();
    const publicKey = keyPair.publicKey;
    const privateKey = keyPair.secretKey;
    console.log("Key pair generated");

    // Generate random blinding factor
    const blindingFactor = nacl.randomBytes(32);
    console.log("Blinding factor generated");

    // Blind the public key
    const blindedPublicKey = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      blindedPublicKey[i] = publicKey[i] ^ blindingFactor[i];
    }
    console.log("Public key blinded");

    // Send blinded public key to server for signing
    console.log("Sending request to server");
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

    console.log("Received response from server");
    const data = await response.json();
    if (!data.blind_signature) {
      if (data.message === "CERTIFICATE_ALREADY_SIGNED") {
        showError('Certificate already signed for this payment.');
        return;
      }
      throw new Error('No blind signature received from server');
    }

    console.log("Blind signature received");
    const blindSignature = base64ToBuffer(data.blind_signature);

    // Unblind the signature
    const unblindedSignature = new Uint8Array(64);
    for (let i = 0; i < 32; i++) {
      unblindedSignature[i] = blindSignature[i] ^ blindingFactor[i];
    }
    for (let i = 32; i < 64; i++) {
      unblindedSignature[i] = blindSignature[i];
    }
    console.log("Signature unblinded");

    // Create the ghostkey certificate
    const ghostkeyCertificate = createGhostkeyCertificate(publicKey, unblindedSignature, data.delegate_info);

    console.log("Calling displayCertificate");
    displayCertificate(publicKey, privateKey, ghostkeyCertificate);
  } catch (error) {
    console.error("Error in generateAndSignCertificate:", error);
    showError('Error generating certificate: ' + error.message);
  }
}

function createGhostkeyCertificate(publicKey, unblindedSignature, delegateInfo) {
  const certificateData = {
    publicKey: bufferToBase64(publicKey),
    unblindedSignature: bufferToBase64(unblindedSignature),
    delegateCertificate: delegateInfo.certificate,
    amount: delegateInfo.amount
  };

  return JSON.stringify(certificateData);
}

function generateTestCertificate() {
  const publicKey = nacl.randomBytes(32);
  const privateKey = nacl.randomBytes(64);
  const unblindedSignature = nacl.randomBytes(64);

  displayCertificate(publicKey, privateKey, unblindedSignature);
}

function displayCertificate(publicKey, privateKey, ghostkeyCertificate) {
  console.log("Displaying certificate");
  try {
    // Armor the ghostkey certificate and private key
    const armoredCertificate = `-----BEGIN FREENET GHOSTKEY CERTIFICATE-----
${wrapBase64(btoa(ghostkeyCertificate), 64)}
-----END FREENET GHOSTKEY CERTIFICATE-----`;

    const armoredPrivateKey = `-----BEGIN FREENET GHOSTKEY PRIVATE KEY-----
${wrapBase64(bufferToBase64(privateKey), 64)}
-----END FREENET GHOSTKEY PRIVATE KEY-----`;

    // Combine certificate and private key
    const combinedKey = `${armoredCertificate}\n\n${armoredPrivateKey}`;

    // Display the combined key if the element exists
    const combinedKeyElement = document.getElementById('combinedKey');
    if (combinedKeyElement) {
      combinedKeyElement.value = combinedKey;
    } else {
      console.warn("Combined key textarea not found. Skipping display.");
    }
    
    const certificateSection = document.getElementById('certificateSection');
    const certificateInfo = document.getElementById('certificate-info');
    
    if (!certificateSection || !certificateInfo) {
      console.error("Certificate section or info element not found");
      throw new Error("Certificate section or info element not found");
    }
    
    certificateSection.style.display = 'block';
    certificateInfo.style.display = 'none';

    // Set up copy button if it exists
    const copyButton = document.getElementById('copyCombinedKey');
    if (copyButton && combinedKeyElement) {
      copyButton.addEventListener('click', function() {
        combinedKeyElement.select();
        document.execCommand('copy');
        this.textContent = 'Copied!';
        setTimeout(() => {
          this.textContent = 'Copy Ghost Key';
        }, 2000);
      });
    } else {
      console.warn("Copy button or combined key textarea not found. Skipping copy functionality.");
    }

    // Verify the certificate
    if (!verifyCertificate(ghostkeyCertificate)) {
      console.error("Certificate verification failed");
      throw new Error("Certificate verification failed");
    }
    
    console.log("Certificate verified and displayed successfully");
  } catch (error) {
    console.error("Error in displayCertificate:", error);
    showError(`Error displaying Ghost Key: ${error.message}. Please contact support.`);
  }
}

function verifyCertificate(ghostkeyCertificate) {
  // In a real implementation, we would verify the ghostkey certificate
  // For now, we'll just check if it's a valid JSON
  try {
    JSON.parse(ghostkeyCertificate);
    return true;
  } catch (error) {
    console.error("Invalid ghostkey certificate:", error);
    return false;
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
