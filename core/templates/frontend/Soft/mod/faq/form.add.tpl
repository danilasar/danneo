<script src="{site_url}/javascript/jquery.form.js"></script>
<script>
  $(document).ready(function() {
      $('#form-link').click(function() {
         $('#form-div').slideToggle();
         return false;
      });
  	  $('#faqask').textarearesizer();
  	  $('#refresh').click(function() {
         var t = new Date().getTime();
         $('#divcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="" />');
      });
    });
</script>
<div class="clear-line"></div>
<div class="site-title"><h3><a id="form-link" href="#">{faq_ask}</a></h3></div>
<div id="form-div" style="display:none">
<form action="{site_url}/index.php?dn=faq" method="post">
<div class="comment">
    <fieldset class="standart">
    <legend>{email_name}</legend>
        <strong>|</strong>
        <input class="width" name="faqauthor" type="text" value="{uname}" required>
    </fieldset>
    <fieldset class="standart">
    <legend>E-Mail:</legend>
        <strong>|</strong>
        <input class="width" name="faqmail" type="text" value="{umail}" required>
    </fieldset>
    <fieldset class="standart">
    <legend>{in_cat}:</legend>
        <strong>|</strong>
        <select class="width" name="catid">
        {sel}
        </select>
    </fieldset>
    <fieldset class="standart">
    <legend>{question}:</legend>
        <textarea class="width" cols="40" rows="5" id="faqask" name="faqask"></textarea>
    </fieldset>
    <!--if:captcha:yes-->
    <fieldset class="standart">
    <legend>Captcha</legend>
    <table class="wpc_100">
    <tbody>
        <tr>
            <td class="wpc_100">
                <strong>|</strong><input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
            </td>
            <td class="ac va pad">
                <div id="divcaptcha"><img src="{site_url}/image.php?to=captcha" alt="" /></div>
            </td>
            <td class="ac va pad">
                <button type="button" id="refresh" class="sub">{all_refresh}</button>
            </td>
        </tr>
    </tbody>
    </table>
    </fieldset>
    <!--if-->
    <!--if:control:yes-->
    <fieldset class="standart">
    <legend>{control_word}</legend>
        <p>{control}</p>
        <strong>|</strong>
        <input class="width" id="respon" name="respon" size="30" type="text" />
        <input name="cid" type="hidden" value="{cid}" />
    </fieldset>
    <!--if-->
    <div class="pad ac">
        <input name="re" type="hidden" value="add" />
        <button type="submit" class="sub">{faq_ask}</button>
    </div>
    <div class="clear"></div>
</div>
</form>
</div>
