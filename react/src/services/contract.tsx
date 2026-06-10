import axios from 'axios'
import { ethers } from 'ethers'
import type { BotSettings } from '../components/trading/types'

export interface SignContractResult {
  success: boolean
  message: string
  insufficientFunds?: boolean
}

export async function signContract(settings: BotSettings): Promise<SignContractResult> {
  if (!window.ethereum) {
    return { success: false, message: 'No wallet found. Please install MetaMask.' }
  }

  let nonce: string
  try {
    const { data } = await axios.get('/api/contracts/nonce')
    nonce = data.nonce
  } catch {
    return { success: false, message: 'Failed to get signing nonce. Please try again.' }
  }

  const provider = new ethers.BrowserProvider(window.ethereum)
  await provider.send('eth_requestAccounts', [])
  const signer = await provider.getSigner()

  const message = JSON.stringify({ nonce, settings })

  let signature: string
  try {
    signature = await signer.signMessage(message)
  } catch {
    return { success: false, message: 'Signature rejected. Please try again.' }
  }

  try {
    await axios.post('/api/contracts/sign', { nonce, message, signature })
  } catch (err: unknown) {
    if (axios.isAxiosError(err) && err.response) {
      const msg = err.response.data as string
      if (err.response.status === 400 && msg === 'Insufficient funds') {
        return { success: false, message: msg, insufficientFunds: true }
      }
      return { success: false, message: msg }
    }
    return { success: false, message: 'Failed to submit contract. Please try again.' }
  }

  return { success: true, message: 'Contract signed successfully!' }
}
