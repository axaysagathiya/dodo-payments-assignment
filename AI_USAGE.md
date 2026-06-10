# AI_USAGE

### Which AI tools you used and for what.
- Use ChatGPT to generate a migration file from the database tables I have written in the design document.
- Use ChatGPT to know what's the best way to implement API authentication.
    - I learnt that we can implement the middleware and we can implement a header extractor. I decided to go with implementing a header extractor, as we would get the business ID using the API key, That we need. With implementing middleware, it would not be possible to get the business ID. 
    
- As I was getting a problem encoding and decoding values while inserting and fetching from the database, I used Gemini CLI to help me fix the code to properly serialize and deserialize the data. 

- After all the implementation, I used Gemini CLI to write all three tests mentioned in the assignment. 
    - The problem I faced was for the tok_timeout and tok_network_error. It was continuously modifying my unknown status with the failed or with the processing, as we have different meanings of failed, processing, and unknown. I chose to go with the unknown status, as the payment is not literally in process or it is not failed either.


### Three decisions you made yourself:
- Database schema 
- File structure
- The format of API
- Where to put Database row lock and unlock in the Rust code, Because when my code failed while running the Concurrency test, Gemini CLI suggested to me to unlock after calling PSP, and I was unlocking before calling PSP, just after setting the invoice payment status as processing. I decided to keep the lock and unlock logic as it is. and  Found one of the conditions in the wrong place. I fixed it.
- Same as I mentioned above, I decided to keep the Invoice payment status unknown in the case of tok_timeout and tok_network error. Because when one payment API is in progress, we set the invoice payment status as processing, and at the end, whatever the result we get, either success, failed, or unknown, we update. When another client Sends the payment request for the same invoice. They can see that this Invoice payment process is in progress. 

### One thing the AI got wrong 
- The problem with AI that frustrated me was its behavior of continuously changing my invoice payment status from unknown to failed or in progress. And I had to fix it multiple times. 
-even when I never ask to modify the actual logic, the AI would go And change my API URL from localhost:invoices/{id} to  localhost:invoice/:id, Which was not working And I had to correct.