import axios from 'axios';
import { ethers } from 'ethers';

declare global {
  interface Window {
    ethereum?: import('ethers').Eip1193Provider;
  }
}

/**
 * Connects with server and authenticates the wallet. 
 * @returns {Promise<boolean>} True if connection, signature, and auth are successful, false otherwise.
 */
export async function connectWallet(): Promise<boolean> {
  try {

    // If no wallet provider is found.
    if (!window.ethereum) {
      throw new Error("NO_WALLET");
    }

    // get the user's wallet address
    const provider = new ethers.BrowserProvider(window.ethereum);
    const accounts = await provider.send('eth_requestAccounts', []);
    const walletAddress = accounts[0];

    const res = await axios.get('/api/user/auth', {
      params: { wallet_address: walletAddress }
    });
    const message = res.data.nonce;

    // Prompt the user to sign the message
    const signer = await provider.getSigner();
    const signature = await signer.signMessage(message);

    // Send the signature back to the server for verification
    const response = await axios.post('/api/user/auth', {
      msg: message,
      signature
    });

    return response.status === 200;

  } catch (error) {
    console.error('Wallet connection or signature failed:', error);
    if (error instanceof Error && error.message === 'NO_WALLET') throw error;
    return false;
  }
}

export async function disconnectWallet(): Promise<void> {
  try {
    const res = await axios.post('/api/user/logout');
    if (res.status === 200) {
      // disconnected successfully
    } else {
      console.warn('Unexpected response during logout:', res);
    }

  } catch (error) {
    console.error('Error during wallet disconnection:', error);
  }
}


