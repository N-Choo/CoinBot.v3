import axios from 'axios'
import { ethers } from 'ethers'

const USDT_ADDRESS = '0xdAC17F958D2ee523a2206206994597C13D831ec7'
let PLATFORM_WALLET = '0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c'

export async function loadConfig(): Promise<void> {
  try {
    const res = await axios.get('/api/config')
    PLATFORM_WALLET = res.data.platform_wallet
  } catch {
    // will try again on first sendCoin call
  }
}

const ERC20_ABI = [
  'function transfer(address to, uint256 value) public returns (bool)',
  'function balanceOf(address owner) public view returns (uint256)',
]

export async function getBalance(): Promise<string> {
  if (!window.ethereum) throw new Error('MetaMask not installed')

  const provider = new ethers.BrowserProvider(window.ethereum)
  // Silent read of already-authorized accounts (no prompt); fall back to an
  // explicit connect request only when there is no authorized account.
  let accounts = await provider.send('eth_accounts', [])
  if (!Array.isArray(accounts) || accounts.length === 0) {
    accounts = await provider.send('eth_requestAccounts', [])
  }
  if (!Array.isArray(accounts) || accounts.length === 0) {
    throw new Error('Wallet not connected')
  }

  const contract = new ethers.Contract(USDT_ADDRESS, ERC20_ABI, provider)
  const balance = await contract.balanceOf(accounts[0])
  return ethers.formatUnits(balance, 6)
}

export async function sendUSDT(amount: string): Promise<boolean> {
  if (!window.ethereum) throw new Error('MetaMask not installed')

  const provider = new ethers.BrowserProvider(window.ethereum)
  await provider.send('eth_requestAccounts', [])
  const signer = await provider.getSigner()
  const sender = await signer.getAddress()

  const network = await provider.getNetwork()
  if (network.chainId !== 1n) {
    throw new Error('Please switch to Ethereum mainnet')
  }

  const amountParsed = ethers.parseUnits(amount, 6)
  const contract = new ethers.Contract(USDT_ADDRESS, ERC20_ABI, provider)
  const balance = await contract.balanceOf(sender)

  if (balance < amountParsed) {
    const formatted = ethers.formatUnits(balance, 6)
    throw new Error(`Insufficient USDT balance: you have ${formatted} USDT`)
  }

  const signerContract = contract.connect(signer) as ethers.Contract
  const tx = await signerContract.transfer(PLATFORM_WALLET, amountParsed)
  await tx.wait()
  await axios.post('/api/transactions/deposit', { tx_hash: tx.hash }, { timeout: 15000 })

  return true
}

export interface ActivityItem {
  id: string
  ticker: string
  amount: string
  status: string
  created_at: string
  tx_hash: string
}

export async function getActivity(): Promise<ActivityItem[]> {
  const res = await axios.get('/api/transactions')
  return res.data.slice(0, 20)
}

export async function withdraw(amount: string): Promise<void> {
  await axios.post('/api/transactions/withdraw', { amount })
}
