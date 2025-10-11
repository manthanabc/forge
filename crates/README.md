# C# Linked List Implementation

This project contains a simple implementation of a linked list in C#, along with an example program demonstrating its usage.

## Files

*   `LinkedList.cs`: Contains the implementation of the `LinkedList<T>` and `Node<T>` classes.
*   `Program.cs`: Contains the `Main` method with an example of how to use the `LinkedList<T>` class.

## How to Compile and Run

To compile and run this code, you will need a C# compiler, such as the one included in the .NET SDK or Mono.

### Using .NET SDK

1.  **Compile the code:**
    ```bash
    csc LinkedList.cs Program.cs -out:LinkedListApp.exe
    ```

2.  **Run the executable:**
    ```bash
    ./LinkedListApp.exe
    ```

### Using Mono

1.  **Compile the code:**
    ```bash
    mcs LinkedList.cs Program.cs -out:LinkedListApp.exe
    ```

2.  **Run the executable:**
    ```bash
    mono LinkedListApp.exe
    ```

## Expected Output

When you run the program, you should see the following output:

```
Initial list:
1 -> 2 -> 3 -> 4 -> 5 -> null
List after removing 3:
1 -> 2 -> 4 -> 5 -> null
```
