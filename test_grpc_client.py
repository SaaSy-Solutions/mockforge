#!/usr/bin/env python3
"""
Simple gRPC client to test MockForge gRPC service
Requires: pip install grpcio grpcio-tools
"""

import grpc
import sys
import os

# Add the generated proto path to Python path
sys.path.append('target/debug/build/mockforge-grpc-*/out/')

try:
    # Try to import generated gRPC code
    from mockforge.greeter import greeter_pb2, greeter_pb2_grpc
    
    def test_grpc_service():
        # Connect to the gRPC server
        with grpc.insecure_channel('127.0.0.1:50051') as channel:
            stub = greeter_pb2_grpc.GreeterStub(channel)
            
            # Create a request
            user_info = greeter_pb2.UserInfo(
                user_id="test-123",
                email="test@example.com",
                status=greeter_pb2.Status.ACTIVE
            )
            
            request = greeter_pb2.HelloRequest(
                name="MockForge Test",
                user_info=user_info,
                tags=["test", "mockforge", "grpc"]
            )
            
            # Make the call
            print("Sending gRPC request to MockForge...")
            try:
                response = stub.SayHello(request)
                print(f"✅ Success! Response: {response.message}")
                if response.metadata:
                    print(f"   Metadata: {response.metadata}")
                if response.items:
                    print(f"   Items: {response.items}")
            except grpc.RpcError as e:
                print(f"❌ gRPC Error: {e.code()} - {e.details()}")
                
    if __name__ == "__main__":
        test_grpc_service()
        
except ImportError:
    print("❌ Could not import generated gRPC code.")
    print("This is expected - the Python client would need the proto files compiled for Python.")
    print("Use grpcurl instead or test via other means.")