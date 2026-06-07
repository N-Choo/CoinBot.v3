
import axios from 'axios'
import { ethers } from 'ethers'

let RECIPIENT_ADDRESS = "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c"

export async function loadConfig(): Promise<void> {
  try {
    const res = await axios.get('/api/config')
    RECIPIENT_ADDRESS = res.data.platform_wallet
  } catch {
    // use default
  }
}

export type CoinSymbol = 'eth' | 'usdt' | 'usdc'

export interface CoinConfig {
  symbol: CoinSymbol
  name: string
  label: string
  address?: string
  decimals: number
}

export const COINS: Record<CoinSymbol, CoinConfig> = {
  eth: { symbol: 'eth', name: 'Ethereum', label: 'ETH', decimals: 18 },
  usdt: { symbol: 'usdt', name: 'Tether', label: 'USDT', decimals: 6, address: '0xdAC17F958D2ee523a2206206994597C13D831ec7' },
  usdc: { symbol: 'usdc', name: 'USD Coin', label: 'USDC', decimals: 6, address: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48' },
}

const ERC20_ABI = [
  "function transfer(address to, uint256 value) public returns (bool)",
  "function balanceOf(address owner) public view returns (uint256)",
]

export async function getBalance(coin: CoinSymbol): Promise<string> {
  if (!window.ethereum) throw new Error("MetaMask not installed")

  const provider = new ethers.BrowserProvider(window.ethereum)
  await provider.send("eth_requestAccounts", [])
  const signer = await provider.getSigner()
  const sender = await signer.getAddress()

  const cfg = COINS[coin]

  if (coin === 'eth') {
    const balance = await provider.getBalance(sender)
    return ethers.formatEther(balance)
  }

  const contract = new ethers.Contract(cfg.address!, ERC20_ABI, provider)
  const balance = await contract.balanceOf(sender)
  return ethers.formatUnits(balance, cfg.decimals)
}

export async function sendCoin(amount: string, coin: CoinSymbol): Promise<boolean> {
  if (!window.ethereum) throw new Error("MetaMask not installed")

  const provider = new ethers.BrowserProvider(window.ethereum)
  await provider.send("eth_requestAccounts", [])
  const signer = await provider.getSigner()
  const sender = await signer.getAddress()

  const network = await provider.getNetwork()
  if (network.chainId !== 1n) {
    throw new Error(`Wrong network: please switch to Ethereum mainnet (currently on chain ID ${network.chainId})`)
  }

  const cfg = COINS[coin]
  const amountParsed = ethers.parseUnits(amount, cfg.decimals)

  if (coin === 'eth') {
    const balance = await provider.getBalance(sender)
    if (balance < amountParsed) {
      throw new Error(`Insufficient ETH balance: you have ${ethers.formatEther(balance)} ETH`)
    }
    const tx = await signer.sendTransaction({ to: RECIPIENT_ADDRESS, value: amountParsed })
    await tx.wait()
    await axios.post('/api/transactions/deposit', { tx_hash: tx.hash })
  } else {
    const contract = new ethers.Contract(cfg.address!, ERC20_ABI, provider)
    const balance = await contract.balanceOf(sender)
    if (balance < amountParsed) {
      const formatted = ethers.formatUnits(balance, cfg.decimals)
      throw new Error(`Insufficient ${cfg.name} balance: you have ${formatted} ${cfg.label}`)
    }
    const signerContract = contract.connect(signer) as ethers.Contract
    const tx = await signerContract.transfer(RECIPIENT_ADDRESS, amountParsed)
    await tx.wait()
    await axios.post(`/api/transactions/deposit`, { tx_hash: tx.hash })
  }

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
