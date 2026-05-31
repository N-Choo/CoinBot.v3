
import { ethers } from 'ethers'

const USDT_ADDRESS = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
const RECIPIENT_ADDRESS = "0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c"

const ERC20_ABI = [
  "function transfer(address to, uint256 value) public returns (bool)",
  "function balanceOf(address owner) public view returns (uint256)",
  "function decimals() public view returns (uint8)"
]

export async function sendUSDT(amountInUSD: string) {
  if (!window.ethereum) throw new Error("MetaMask not installed")

  const provider = new ethers.BrowserProvider(window.ethereum)
  await provider.send("eth_requestAccounts", [])
  const signer = await provider.getSigner()
  const sender = await signer.getAddress()

  const network = await provider.getNetwork()
  if (network.chainId !== 1n) {
    throw new Error(`Wrong network: please switch to Ethereum mainnet (currently on chain ID ${network.chainId})`)
  }

  const contract = new ethers.Contract(USDT_ADDRESS, ERC20_ABI, provider)
  const balance = await contract.balanceOf(sender)
  const amount = ethers.parseUnits(amountInUSD, 6)

  if (balance < amount) {
    const formatted = ethers.formatUnits(balance, 6)
    throw new Error(`Insufficient USDT balance: you have ${formatted} USDT but attempted to send ${amountInUSD} USDT`)
  }

  const signerContract = contract.connect(signer) as ethers.Contract
  const tx = await signerContract.transfer(RECIPIENT_ADDRESS, amount)
  await tx.wait()
  return tx.hash
}
